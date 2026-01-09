# Orchids Manager 🌸

<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" height="128" alt="Orchids Manager Logo">
</p>

<p align="center">
  <strong>Next-Generation Account Automation & Management Tool</strong>
  <br>
  <em>Secure, Fast, and Robust</em>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#architecture-v2">Architecture V2</a> •
  <a href="#installation">Installation</a> •
  <a href="#development">Development</a>
</p>

---

## Features 🚀

### 🤖 **Advanced Automation (V2)**

- **Electron Injection**: Directly injects control scripts into the `Orchids.app` process, bypassing brittle UI scripting.
- **WebView Registration**: Built-in incognito webview for rapid account registration via Clerk.
- **Auto-Verification**:
  - **IMAP**: Standard protocols for any mail provider.
  - **Google OAuth**: Native Gmail API integration, supports **Custom Domains** (Google Workspace).
- **Session Capture**: real-time monitoring of the `Cookies` SQLite database to detect logins instantly.

### 👥 **Smart Account Management**

- **Multi-Account Vault**: Securely store and manage unlimited Orchids accounts.
- **Pro Dashboard**: View total credits, average balance, and identify "Low Balance" accounts at a glance.
- **Smart Switch**: One-click context switching. Automatically finds and switches to your highest-balance account.
- **Session Persistence**: Robust restoration logic ensures you stay logged in even after switching.

### 🔧 **Power Tools**

- **API Proxy**: Built-in local proxy for inspecting or redirecting app traffic.
- **Debug Console**: Real-time logs for monitoring automation steps and injection status.

---

## Architecture V2 🏗️

Orchids Manager V2 moves away from fragile AppleScript macros to a robust **Process Injection** model.

### 1. **Session Hijack & Injection**

Instead of simulating clicks, we patch the `main/index.js` of the target Electron app. This allows us to:

- Read/Write `Cookies` directly at the session level.
- Clear `localStorage` and `cache` programmatically to ensure clean state transitions.
- Force-reload windows to apply new sessions instantly.

### 2. **Database Monitoring**

We no longer rely on deep links or URL callbacks.

- A background service watches `~/Library/Application Support/Orchids/Cookies` (SQLite).
- When a new `__session` cookie appears, we decode the JWT, fetch the user profile, and save it to our local vault automatically.

### 3. **Robust Switching Flow**

Switching accounts is a delicate process to prevent the "Logged Out" state:

1.  **Kill** the Orchids process.
2.  **Write** the target session cookies to a restoration file.
3.  **Launch** Orchids.
4.  **Inject**: The startup script detects the restoration file.
    _ Clears **Cache** & **Cookies** (Filesystem).
    _ Force-clears **LocalStorage** (Renderer).
    _ Sets new Cookies.
    _ Reloads the window.
    This ensures a 100% success rate when swapping users.

---

## Installation 📦

### Prerequisites

- macOS 10.15+ (Apple Silicon or Intel)
- **Orchids.app** installed in `/Applications`

### Setup

1.  Download the latest `.dmg` from the [Releases](https://github.com/hua123an/orchids-manager/releases) page.
2.  Drag to **Applications**.
3.  Launch **Orchids Manager**.

> **Note**: On first run, the app may ask for permission to modify the Orchids application files. This is required for the injection mechanism to work.

---

## Development 🛠️

### Tech Stack

- **Frontend**: Vue 3, Vite, TailwindCSS (Dark Mode supported)
- **Backend**: Rust (Tauri V2)
- **Database**: SQLite (via `rusqlite`), JSON (for local vault)
- **Automation**: Node.js (Injection Scripts), Native Rust Threads

### Quick Start

1.  **Install Dependencies**

    ```bash
    npm install
    ```

2.  **Run Development Environment**

    ```bash
    npm run tauri dev
    ```

    _This starts the frontend HMR server and the Rust backend._

3.  **Build for Production**
    ```bash
    npm run tauri build
    ```
    _Artifacts located in `src-tauri/target/release/bundle/dmg/`_

### Project Structure

```
orchids-manager/
├── src/                    # Vue 3 Frontend
│   ├── App.vue             # Main UI Logic
│   └── assets/             # Styles & Icons
├── src-tauri/              # Rust Backend
│   ├── src/
│   │   ├── lib.rs          # Main Command Handlers
│   │   ├── capture_service.rs # Cookies DB Monitoring
│   │   ├── injection.rs    # Electron Injection Logic
│   │   └── assets/         # JS Payloads (injection.js)
│   └── tauri.conf.json     # App Config
└── package.json
```

---

## License 📄

MIT License © 2026 Orchids Manager Team

<p align="center">
  <em>Automate responsibly.</em>
</p>
