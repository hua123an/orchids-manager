// --- ORCHIDS MANAGER INJECTION START V2.6 ---
(function () {
  if (global.__ORCHIDS_MANAGER_INJECTED__) {
      console.log("[OrchidsManager] Script already injected, skipping.");
      return;
  }
  global.__ORCHIDS_MANAGER_INJECTED__ = true;

  const fs = require("fs");
  const path = require("path");
  const { session, app } = require("electron");

  // Configuration - Hardcoded shared path for reliability
  const MANAGER_DATA_DIR = path.join(
    process.env.HOME,
    "Library/Application Support/OrchidsManager"
  );
  const CAPTURED_FILE = path.join(MANAGER_DATA_DIR, "captured_session.json");
  const RESTORE_FILE = path.join(MANAGER_DATA_DIR, "restore_session.json");

  // Ensure directory exists
  if (!fs.existsSync(MANAGER_DATA_DIR)) {
    try {
      fs.mkdirSync(MANAGER_DATA_DIR, { recursive: true });
    } catch (e) {}
  }

  // DEBUG LOGGING
  const LOG_FILE = path.join(process.env.HOME, "Desktop", "orchids_debug.log");
  function log(msg) {
    try {
      fs.appendFileSync(LOG_FILE, `[${new Date().toISOString()}] ${msg}\n`);
    } catch (e) {}
  }

  // Helper to read Machine ID
  function getMachineId() {
      try {
          // App UserData: ~/Library/Application Support/Orchids
          const appDataDir = app.getPath('userData');
          const updaterIdPath = path.join(appDataDir, '.updaterId');
          if (fs.existsSync(updaterIdPath)) {
              return fs.readFileSync(updaterIdPath, 'utf8').trim();
          }
      } catch(e) {
          log("Failed to read machine ID: " + e.message);
      }
      return null;
  }

  log("Injection V2.6 script started!");
  log("Data dir: " + MANAGER_DATA_DIR);

  let isDebouncing = false;

  // --- CAPTURE LOGIC ---
  async function checkCurrentSession() {
    const ses = session.defaultSession;
    if (!ses) return;

    try {
      // Check if we have a session to capture
      const cookies = await ses.cookies.get({ name: "__session" });
      if (cookies.length > 0) {
        log(
          "Initial check: Found existng __session cookie, triggering capture"
        );
        processCookieCapture(ses);
      }
    } catch (e) {
      log("Initial check verification failed: " + e.message);
    }
  }

  async function processCookieCapture(ses) {
    if (isDebouncing) return;
    isDebouncing = true;

    setTimeout(async () => {
      try {
        // Verify session exists
        const cookies = await ses.cookies.get({});
        const sessionCookie = cookies.find((c) => c.name === "__session");

        if (!sessionCookie) {
          isDebouncing = false;
          return;
        }

        const parseJwt = (token) => {
          try {
            return JSON.parse(
              Buffer.from(token.split(".")[1], "base64").toString()
            );
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

        // Avoid capturing the session we just restored
        if (fs.existsSync(RESTORE_FILE)) {
          try {
            const restoreData = JSON.parse(
              fs.readFileSync(RESTORE_FILE, "utf8")
            );
            if (restoreData.id === userId) {
              log("Ignoring capture of just-restored session");
              // Only delete if successfully matched
              // fs.unlinkSync(RESTORE_FILE); // Maybe keeping it is safer?
              isDebouncing = false;
              return;
            }
          } catch (e) {}
        }

        // Capture LocalStorage (Requires a window)
        let extractedLocalStorage = null;
        try {
            log("[Capture] Attempting LS capture...");
            let BrowserWindow;
            try {
                 BrowserWindow = require('electron').BrowserWindow;
                 log(`[Capture] BrowserWindow module available: ${!!BrowserWindow}`);
            } catch(e) {
                 log(`[Capture] FAILED to require BrowserWindow: ${e.message}`);
            }

            if (BrowserWindow) {
                // Retry loop to find windows (Increased to 20 attempts for slow cold starts)
                for (let i = 0; i < 20; i++) {
                    const wins = BrowserWindow.getAllWindows();
                    log(`[Capture] Attempt ${i+1}: Found ${wins.length} windows.`);
                    
                    if (wins.length > 0) {
                        for (const win of wins) {
                             try {
                                if (win.isDestroyed()) continue;
                                const jsonLS = await win.webContents.executeJavaScript('JSON.stringify({...localStorage})');
                                const parsed = JSON.parse(jsonLS);
                                if (Object.keys(parsed).length > 0) {
                                    extractedLocalStorage = parsed;
                                    log('Captured LocalStorage from window ID ' + win.id + ' (' + Object.keys(parsed).length + ' keys)');
                                    break;
                                } else {
                                    log(`[Capture] Window ${win.id} returned empty LS.`);
                                }
                             } catch (err) {
                                 log(`[Capture] Window ${win.id} LS access failed: ${err.message}`);
                             }
                        }
                    }
                    
                    if (extractedLocalStorage) break;
                    // Wait 500ms before retry
                    await new Promise(r => setTimeout(r, 500));
                }
            } else {
                 log("[Capture] Skipping LS capture because BrowserWindow is missing.");
            }
            
            if (!extractedLocalStorage) {
                log("[Capture] Failed to capture LocalStorage after retries.");
            }
            
        } catch(e) {
            log('LS Capture Warning: ' + e.message);
        }

        // Fetch Profile (Restored)
        const profileUrl = `https://orchids-server.calmstone-6964e08a.westeurope.azurecontainerapps.io/user/profile/${userId}`;
        const response = await fetch(profileUrl, {
          headers: {
            Authorization: `Bearer ${sessionCookie.value}`,
            "Content-Type": "application/json",
          },
        });

        if (!response.ok) throw new Error("Profile fetch failed");
        const userProfile = await response.json();

        if (!extractedLocalStorage && fs.existsSync(CAPTURED_FILE)) {
             try {
                 const existingData = JSON.parse(fs.readFileSync(CAPTURED_FILE, 'utf8'));
                 if (existingData.id === userId && existingData.local_storage) {
                     log(`[Capture] Preserving existing LocalStorage for ${userId}`);
                     extractedLocalStorage = existingData.local_storage;
                 }
             } catch(e) {}
        }

        const machineId = getMachineId();
        if (machineId) log("Captured Machine ID: " + machineId);

        const accountData = {
          machine_id: machineId,
          id: userId,
          display_name: userProfile.fullName || "Unknown",
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
            credits: userProfile.credits,
          },
          cookies: cookies.map((c) => ({
            name: c.name,
            value: c.value,
            domain: c.domain,
            path: c.path,
            secure: c.secure,
            http_only: c.httpOnly,
            expiration_date: c.expirationDate,
          })),
          local_storage: extractedLocalStorage
        };

        // Write
        const jsonStr = JSON.stringify(accountData, null, 2);
        fs.writeFileSync(CAPTURED_FILE, jsonStr);
        log("[OrchidsManager] Session captured for " + userId);
      } catch (error) {
        log("Capture failed: " + error.message);
      } finally {
        isDebouncing = false;
      }
    }, 1000);
  }

  function setupSessionCapture() {
    const ses = session.defaultSession;
    if (!ses) return;
    checkCurrentSession();
    ses.cookies.on("changed", (event, cookie, cause, removed) => {
      if (removed || cookie.name !== "__session") return;
      processCookieCapture(ses);
    });
  }

  // --- RESTORE LOGIC ---
  async function checkAndRestoreSession() {
    if (!fs.existsSync(RESTORE_FILE)) {
      log("No restore file found at " + RESTORE_FILE);
      return;
    }

    log("Restore file FOUND. Starting restore sequence...");

    try {
      const accountData = JSON.parse(fs.readFileSync(RESTORE_FILE, "utf8"));
      const ses = session.defaultSession;

      // 1. Clear Cookies only - DISABLED to preserve device client tokens
      // await ses.clearStorageData({ storages: ['cookies'] });
      // log('Cookies cleared (LocalStorage preserved).');

      // 2. Restore
      let count = 0;
      for (const cookie of accountData.cookies) {
        // Skip __client cookie to avoid invalidating the session with an old device token
        if (cookie.name === "__client") {
            continue;
        }

        const cleanCookie = {
          url:
            (cookie.secure ? "https://" : "http://") +
            cookie.domain.replace(/^\./, ""),
          name: cookie.name,
          value: cookie.value,
          domain: cookie.domain,
          path: cookie.path,
          secure: cookie.secure,
          httpOnly: cookie.http_only || cookie.httpOnly,
          expirationDate: cookie.expiration_date || cookie.expirationDate,
        };
        try {
          await ses.cookies.set(cleanCookie);
          count++;
        } catch (e) {
          log(`Cookie set error (${cookie.name}): ${e.message}`);
        }
      }

      log(`Restored ${count} cookies for ${accountData.display_name}`);

      // 3. Delete Restore File safely
      try { fs.unlinkSync(RESTORE_FILE); } catch(e) {}

      // 4. Reload Windows with Polling (Race Condition Fix)
      const { BrowserWindow } = require("electron");

      let attempts = 0;
      const maxAttempts = 20; // 10 seconds

      const reloadInterval = setInterval(() => {
        const windows = BrowserWindow.getAllWindows();
        if (windows.length > 0) {
          windows.forEach((win) => {
            log("Restoring LocalStorage & Reloading...");
            
            let script = "";

            try {
                const lsData = accountData.local_storage;
                log(`[Restore] LS Data Type: ${typeof lsData}, IsNull: ${lsData === null}`);

                if (lsData && Object.keys(lsData).length > 0) {
                    const ky = Object.keys(lsData).length;
                    log(`[Restore] Injecting ${ky} LS keys and redirecting to /`);
                    
                    script = `try {
                        const ls = ${JSON.stringify(lsData)};
                        localStorage.clear();
                        Object.entries(ls).forEach(([k,v]) => localStorage.setItem(k,v));
                        // FORCE NAVIGATION TO HOME TO TRIGGER AUTH CHECK
                        window.location.href = '/'; 
                    } catch(e) { console.error(e); }`;
                } else {
                    log("[Restore] No LS data found. Just reloading.");
                    // Still might want to force home?
                    // If we have cookies but no LS, maybe just reload is safer?
                    // Let's try forcing home anyway if we have cookies.
                    script = `window.location.href = '/';`;
                }
            } catch(e) {
                log("[Restore] Error preparing script: " + e.message);
            }
            
            if (script) {
                win.webContents.executeJavaScript(script).then(() => {
                    win.reload();
                }).catch(() => win.reload());
            } else {
                win.reload();
            }
          });
          clearInterval(reloadInterval);
        } else {
          attempts++;
          if (attempts >= maxAttempts) {
            log("No windows found to reload after 10s.");
            clearInterval(reloadInterval);
          }
        }
      }, 500);

      log("Cookie restore complete. Waiting for window to reload...");
    } catch (e) {
      log("Restore CRITICAL ERROR: " + e.message);
    }
  }

  // Initialization
  const onReady = () => {
    checkAndRestoreSession().then(() => {
      setupSessionCapture();
    });
  };

  if (app.isReady()) {
    onReady();
  } else {
    app.on("ready", onReady);
  }
})();
// --- ORCHIDS MANAGER INJECTION END V2 ---
