// --- ORCHIDS MANAGER INJECTION START V2 ---
(function () {
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

  log("Injection V2 script started!");
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

        // Fetch Profile
        const profileUrl = `https://orchids-server.calmstone-6964e08a.westeurope.azurecontainerapps.io/user/profile/${userId}`;
        const response = await fetch(profileUrl, {
          headers: {
            Authorization: `Bearer ${sessionCookie.value}`,
            "Content-Type": "application/json",
          },
        });

        if (!response.ok) throw new Error("Profile fetch failed");
        const userProfile = await response.json();

        const accountData = {
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

      // 1. Clear Cookies & Cache (Ensure clean slate)
      await ses.clearCache();
      await ses.clearStorageData({ storages: ['cookies'] });
      log('Cache and Cookies cleared (LocalStorage preserved).');

      // 2. Restore
      let count = 0;
      for (const cookie of accountData.cookies) {
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

      // 3. Delete Restore File (So we don't loop restore on next boot if we crash)
      fs.unlinkSync(RESTORE_FILE);

      // 4. Reload Windows with Polling (Race Condition Fix)
      const { BrowserWindow } = require("electron");

      let attempts = 0;
      const maxAttempts = 20; // 10 seconds

      const reloadInterval = setInterval(() => {
        const windows = BrowserWindow.getAllWindows();
        if (windows.length > 0) {
          windows.forEach((win) => {
            log("Clearing renderer storage and reloading window...");
            // Force clear renderer storage to remove old user artifacts
            // This combined with new cookies should force a clean login
            win.webContents.executeJavaScript('try { localStorage.clear(); sessionStorage.clear(); } catch(e){}').then(() => {
                win.reload();
            }).catch(() => win.reload());
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
