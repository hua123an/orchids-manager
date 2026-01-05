// --- ORCHIDS MANAGER INJECTION START ---
(function() {
  const fs = require('fs');
  const path = require('path');
  const { session, app } = require('electron');

  // Configuration
  const MANAGER_DATA_DIR = path.join(process.env.HOME, 'Library/Application Support/com.huaan.orchids-manager');
  const CAPTURED_FILE = path.join(MANAGER_DATA_DIR, 'captured_session.json');
  const RESTORE_FILE = path.join(MANAGER_DATA_DIR, 'restore_session.json');
  
  // Ensure directory exists
  if (!fs.existsSync(MANAGER_DATA_DIR)) {
    try {
        fs.mkdirSync(MANAGER_DATA_DIR, { recursive: true });
    } catch(e) {}
  }

  // DEBUG LOGGING
  const LOG_FILE = path.join(process.env.HOME, 'Desktop', 'orchids_debug.log');
  function log(msg) {
    try {
        fs.appendFileSync(LOG_FILE, `[${new Date().toISOString()}] ${msg}\n`);
    } catch(e) {}
  }
  
  log('Injection script started!');
  log('Data dir: ' + MANAGER_DATA_DIR);

  let isDebouncing = false;

  // --- CAPTURE LOGIC ---
  async function checkCurrentSession() {
      const ses = session.defaultSession;
      if (!ses) return;
      
      try {
          const cookies = await ses.cookies.get({ name: '__session' });
          if (cookies.length > 0) {
              log('Initial check: Found existng __session cookie, triggering capture');
              processCookieCapture(ses);
          }
      } catch (e) {
          log('Initial check failed: ' + e.message);
      }
  }

  async function processCookieCapture(ses) {
      if (isDebouncing) return;
      isDebouncing = true;

      // Wait 1 second for other cookies (client_uat, etc) to settle
      setTimeout(async () => {
        try {
          // Double check if we still have the session
          const cookies = await ses.cookies.get({});
          log(`Got ${cookies.length} cookies`);
          
          const sessionCookie = cookies.find(c => c.name === '__session');
          
          if (!sessionCookie) {
              log('Session cookie missing after wait');
              isDebouncing = false;
              return;
          };

          // Basic JWT decode to get ID (avoid external deps if possible)
          const parseJwt = (token) => {
            try {
              return JSON.parse(Buffer.from(token.split('.')[1], 'base64').toString());
            } catch (e) {
              return null;
            }
          };

          const jwtPayload = parseJwt(sessionCookie.value);
          if (!jwtPayload || !jwtPayload.sub) {
             isDebouncing = false;
             return;
          }

          const userId = jwtPayload.sub;

          // Check if this is the user we just restored to avoid capture loop
          // (Simple check: if we have a restore file and it matches this user)
          if (fs.existsSync(RESTORE_FILE)) {
              try {
                  const restoreData = JSON.parse(fs.readFileSync(RESTORE_FILE, 'utf8'));
                  if (restoreData.id === userId) {
                      // It's the one we just restored, cleanup and ignore
                      fs.unlinkSync(RESTORE_FILE);
                      isDebouncing = false;
                      return; 
                  }
              } catch(e) {}
          }

          // Fetch User Profile
          const profileUrl = `https://orchids-server.calmstone-6964e08a.westeurope.azurecontainerapps.io/user/profile/${userId}`;
          
          const response = await fetch(profileUrl, {
             headers: {
                 'Authorization': `Bearer ${sessionCookie.value}`,
                 'Content-Type': 'application/json'
             }
          });
          
          if (!response.ok) throw new Error('Profile fetch failed');
          const userProfile = await response.json();

          const accountData = {
            id: userId,
            display_name: userProfile.fullName || 'Unknown',
            email: userProfile.email,
            avatar_url: userProfile.imageUrl,
            last_active_at: Date.now(),
            user_info: {
                id: null,
                user_id: userProfile.userId,
                full_name: userProfile.fullName,
                email: userProfile.email,
                image_url: userProfile.imageUrl,
                plan: userProfile.plan,
                credits: userProfile.credits
            },
            cookies: cookies.map(c => ({
                name: c.name,
                value: c.value,
                domain: c.domain,
                path: c.path,
                secure: c.secure,
                http_only: c.httpOnly,
                expiration_date: c.expirationDate
            }))
          };

          // Write to captured file
          const jsonStr = JSON.stringify(accountData, null, 2);
          log(`Writing ${jsonStr.length} bytes to ${CAPTURED_FILE}`);
          fs.writeFileSync(CAPTURED_FILE, jsonStr);
          log('[OrchidsManager] Session captured for ' + userId);

        } catch (error) {
          log('[OrchidsManager] Capture failed: ' + error.message + '\n' + error.stack);
          console.error('[OrchidsManager] Capture failed:', error);
        } finally {
            isDebouncing = false;
        }
      }, 1000);
  }

  function setupSessionCapture() {
    const ses = session.defaultSession;
    if (!ses) return;

    // Check immediately
    checkCurrentSession();

    ses.cookies.on('changed', (event, cookie, cause, removed) => {
      // Only care about session cookie being added/updated
      if (removed || cookie.name !== '__session') return;
      
      log('Detected __session cookie change!');
      processCookieCapture(ses);
    });
  }

  // --- RESTORE LOGIC ---
  async function checkAndRestoreSession() {
      if (!fs.existsSync(RESTORE_FILE)) return;

      console.log('[OrchidsManager] Restore file found, attempting restore...');
      
      try {
          const accountData = JSON.parse(fs.readFileSync(RESTORE_FILE, 'utf8'));
          const ses = session.defaultSession;
          
          // 1. Clear current session
          await ses.clearStorageData({ storages: ['cookies', 'localstorage'] });
          console.log('[OrchidsManager] Cleared existing session');

          // 2. Restore cookies
          for (const cookie of accountData.cookies) {
              // Electron keys for set() are slightly different (camelCase vs snake_case sometimes)
              // We need to construct a clean object
              const cleanCookie = {
                  url: (cookie.secure ? 'https://' : 'http://') + cookie.domain.replace(/^\./, ''),
                  name: cookie.name,
                  value: cookie.value,
                  domain: cookie.domain,
                  path: cookie.path,
                  secure: cookie.secure,
                  httpOnly: cookie.http_only || cookie.httpOnly,
                  expirationDate: cookie.expiration_date || cookie.expirationDate
              };
              
              try {
                  await ses.cookies.set(cleanCookie);
              } catch(e) {
                  console.warn(`[OrchidsManager] Failed to set cookie ${cookie.name}:`, e.message);
              }
          }
          
          console.log(`[OrchidsManager] Successfully restored session for ${accountData.display_name}`);
          
          // 3. Mark as pending reload so we don't delete file immediately
          // (Handled by capture check above) or just keep it there until next capture overwrites it
          
          // Force reload all windows
          const { BrowserWindow } = require('electron');
          BrowserWindow.getAllWindows().forEach(win => {
              win.reload();
          });

      } catch (e) {
          console.error('[OrchidsManager] Restore failed:', e);
      }
  }

  // Initialization
  const onReady = () => {
      // 1. Check if we need to restore immediately on boot
      checkAndRestoreSession();
      
      // 2. Start monitoring for new logins
      setupSessionCapture();
      
      // 3. Watch for restore file changes (runtime switching)
      fs.watchFile(RESTORE_FILE, { interval: 1000 }, (curr, prev) => {
          if (curr.mtime !== prev.mtime && fs.existsSync(RESTORE_FILE)) {
              checkAndRestoreSession();
          }
      });
  };

  if (app.isReady()) {
      onReady();
  } else {
      app.on('ready', onReady);
  }

})();
// --- ORCHIDS MANAGER INJECTION END ---
