<script setup>
import { ref, onMounted, computed, nextTick, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// --- State ---
const accounts = ref([]);
const activeTab = ref(localStorage.getItem('active_tab') || "dashboard");

watch(activeTab, (newTab) => {
    localStorage.setItem('active_tab', newTab);
});

const isListening = ref(false);
const statusMsg = ref("Ready");
const showToast = ref(false);
const toastMsg = ref("");
const debugLogs = ref([]);
const activeAccountId = ref(null);
const showLogoutModal = ref(false);

const proxyRunning = ref(false);
const proxyPort = ref(3000);
const proxyStatus = ref("Stopped");
const proxyUrl = computed(() => `http://127.0.0.1:${proxyPort.value}`);

// Automation State
const autoMode = ref("login"); // login | register
const autoEmail = ref("");
const autoPass = ref("");
const imapHost = ref("imap.gmail.com");
const imapPort = ref(993);
const imapUser = ref("");
const imapPass = ref("");
const autoLogs = ref([]);
const autoRunning = ref(false);

// Gmail OAuth State
const gmailOAuthStatus = ref("Not authenticated");
const gmailOAuthLoading = ref(false);

// --- Icons ---
const Icons = {
  Home: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path><polyline points="9 22 9 12 15 12 15 22"></polyline></svg>` },
  Users: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path><circle cx="9" cy="7" r="4"></circle><path d="M23 21v-2a4 4 0 0 0-3-3.87"></path><path d="M16 3.13a4 4 0 0 1 0 7.75"></path></svg>` },
  Globe: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="2" y1="12" x2="22" y2="12"></line><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path></svg>` },
  Settings: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>` },
  Plus: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>` },
  Download: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line></svg>` },
  Zap: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon></svg>` },
  Star: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"></polygon></svg>` },
  Check: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>` },
  Alert: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>` },
  Sun: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"></circle><line x1="12" y1="1" x2="12" y2="3"></line><line x1="12" y1="21" x2="12" y2="23"></line><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line><line x1="1" y1="12" x2="3" y2="12"></line><line x1="21" y1="12" x2="23" y2="12"></line><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line></svg>` },
  Trash: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>` },
  Play: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="5 3 19 12 5 21 5 3"></polygon></svg>` },
  Stop: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect></svg>` },
  Terminal: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line></svg>` },
  Close: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>` },
  Edit: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg>` },
};

onMounted(async () => {
  await loadAccounts();
  await toggleListener(true);

  import("@tauri-apps/api/event").then(({ listen }) => {
    listen("account-captured", (event) => { showNotification(`Captured: ${event.payload.display_name}`); loadAccounts(); });
    listen("debug-log", (event) => {
      debugLogs.value.unshift(`[${new Date().toLocaleTimeString()}] ${event.payload}`);
      if (debugLogs.value.length > 200) debugLogs.value.pop();
    });
    listen("imap_log", (event) => {
      const now = new Date().toLocaleTimeString();
      autoLogs.value.unshift(`[${now}] ${event.payload}`);
      if (autoLogs.value.length > 100) autoLogs.value.pop();
    });
  });

  // Auto-Monitor Credits (Every 10s)
  setInterval(async () => {
    if (!activeAccountId.value) return;
    try {
        const updated = await invoke("refresh_active_account");
        // Update local
        const idx = accounts.value.findIndex(a => a.id === updated.id);
        if (idx !== -1) accounts.value[idx] = updated;
        
        const credits = updated.user_info?.credits || 0;
        if (credits < 50000) {
            const best = recommendAccounts.value[0];
            if (best && (best.user_info?.credits || 0) > 50000) {
                showNotification(`Low Credits (${formatCredits(credits)}). Switching to ${best.display_name}...`);
                await setActive(best.id);
            }
        }
    } catch (e) {}
  }, 10000);
});

const activeAccount = computed(() => accounts.value.find(a => a.id === activeAccountId.value));
const recommendAccounts = computed(() => accounts.value.filter(a => a.id !== activeAccountId.value).sort((a, b) => (b.user_info?.credits || 0) - (a.user_info?.credits || 0)));
const avgCredits = computed(() => {
    if (accounts.value.length === 0) return 0;
    const sum = accounts.value.reduce((acc, curr) => acc + (curr.user_info?.credits || 0), 0);
    return Math.floor(sum / accounts.value.length);
});
const greetingName = computed(() => {
    if (!activeAccount.value || !activeAccount.value.email) return "Chief";
    return activeAccount.value.email.split('@')[0];
});

function formatCredits(n) {
  if (!n && n !== 0) return "0";
  if (n >= 1000000) return (n / 1000000).toFixed(1) + "M";
  if (n >= 1000) return (n / 1000).toFixed(0) + "k";
  return n;
}

function log(msg) {
    autoLogs.value.unshift(`[${new Date().toLocaleTimeString()}] ${msg}`);
}


// IMAP Profile Logic
const showImapSettings = ref(false);
const imapProfiles = ref([]);
const selectedProfileId = ref("");
const isEditingProfile = ref(false);
const editingProfile = ref({ id: null, name: '', host: '', port: 993, user: '', pass: '' });

function loadTurnstile() {
    if(window.turnstile) return;
    const script = document.createElement('script');
    script.src = "https://challenges.cloudflare.com/turnstile/v0/api.js";
    script.async = true; 
    script.defer = true;
    document.head.appendChild(script);
}

onMounted(() => {
    loadAccounts();
    loadTurnstile();
    const saved = localStorage.getItem('imap_profiles');
    if(saved) {
        try { 
            imapProfiles.value = JSON.parse(saved); 
            // Auto-select if only one profile exists
            if (imapProfiles.value.length === 1) {
                applyProfile(imapProfiles.value[0]);
                selectedProfileId.value = imapProfiles.value[0].id;
            }
        } catch(e){}
    }

    // Auto-test removed. Please use manual start.
});

const emailPrefix = ref("");
const emailSuffix = ref("@huaan666.site");
const customSuffix = ref("");
const isCustomSuffix = computed(() => emailSuffix.value === "@custom");

function updateAutoEmail() {
    const suffix = isCustomSuffix.value ? customSuffix.value : emailSuffix.value;
    autoEmail.value = emailPrefix.value + suffix;
}

function fillTestData() {
   if (emailSuffix.value !== '@huaan666.site') return;
   
   // Username: letters and numbers only (no underscores)
   const rand = Math.random().toString(36).substring(2, 8);
   emailPrefix.value = `huaan${rand}`;
   
   updateAutoEmail();
   
   // Password: letters, numbers, @, .
   autoPass.value = `Huaan.2026@${rand}`; 
}

watch(autoMode, (newMode) => {
    if (newMode === 'register' && !emailPrefix.value && emailSuffix.value === '@huaan666.site') {
        fillTestData();
    }
});

function applyPreset(type) {
    if(type === 'gmail') {
        editingProfile.value.host = 'imap.gmail.com';
        editingProfile.value.port = 993;
        editingProfile.value.name = 'Gmail Config';
    } else if(type === 'qq') {
        editingProfile.value.host = 'imap.qq.com';
        editingProfile.value.port = 993;
        editingProfile.value.name = 'QQ Config';
    } else if(type === '163') {
        editingProfile.value.host = 'imap.163.com';
        editingProfile.value.port = 993;
        editingProfile.value.name = '163 Config';
    }
}

function startNewProfile() {
    editingProfile.value = { id: null, name: '', host: '', port: 993, user: '', pass: '' };
    isEditingProfile.value = true;
}

function editProfile(p) {
    editingProfile.value = { ...p };
    isEditingProfile.value = true;
}

function cancelEdit() {
    isEditingProfile.value = false;
}

function saveProfile() {
    if(!editingProfile.value.name) return; 
    
    if(editingProfile.value.id) {
        const idx = imapProfiles.value.findIndex(x => x.id === editingProfile.value.id);
        if(idx !== -1) imapProfiles.value[idx] = { ...editingProfile.value };
    } else {
        const newId = Date.now().toString();
        imapProfiles.value.push({ ...editingProfile.value, id: newId });
    }
    localStorage.setItem('imap_profiles', JSON.stringify(imapProfiles.value));
    isEditingProfile.value = false;
}

function deleteProfile(id) {
    if(!confirm("Delete this profile?")) return;
    imapProfiles.value = imapProfiles.value.filter(x => x.id !== id);
    localStorage.setItem('imap_profiles', JSON.stringify(imapProfiles.value));
}

function selectProfile(p) {
    applyProfile(p);
    showImapSettings.value = false;
}

function applyProfile(p) {
    imapHost.value = p.host;
    imapPort.value = p.port;
    if(p.user) imapUser.value = p.user;
    if(p.pass) imapPass.value = p.pass;
    log(`Applied IMAP Profile: ${p.name}`);
}

function applyProfileById(id) {
    const p = imapProfiles.value.find(x => x.id === id);
    if(p) applyProfile(p);
}

async function getTurnstileToken() {
    return new Promise((resolve, reject) => {
        if(!window.turnstile) { 
            // Try waiting
            setTimeout(() => {
                if(window.turnstile) resolve(getTurnstileToken());
                else reject("Turnstile not loaded");
            }, 1000);
            return;
        }
        
        try {
            window.turnstile.render('#turnstile-widget', {
                sitekey: '0x4AAAAAAAWXJGBD7bONzLBd', 
                callback: function(token) {
                    resolve(token);
                },
                'error-callback': function() {
                    reject("Turnstile Error");
                },
            });
        } catch(e) { reject(e); }
    });
}

async function startAutomation() {
    if(autoRunning.value) return;
    autoRunning.value = true;
    log(`Starting ${autoMode.value} flow for ${autoEmail.value}...`);
    
    try {
        let res;
        if (autoMode.value === 'login') {
            log("Sending Login Request...");
            res = await invoke("clerk_action_login", { email: autoEmail.value, pass: autoPass.value });
            log("SUCCESS: " + res);
        } else if (autoMode.value === 'register') {
             log("Opening Registration Window...");
             res = await invoke("clerk_action_register_webview", {
                 email: autoEmail.value,
                 pass: autoPass.value,
                 imapHost: imapHost.value || "",
                 imapPort: Number(imapPort.value) || 993,
                 imapUser: imapUser.value || "",
                 imapPass: imapPass.value || ""
             });
             log(res);
             log("Please complete the registration in the new window.");
             // Stop here, user interaction required in popup
             return; 
        } else {
            log("Unknown mode");
            return;
        }
            
            let json;
            try { json = JSON.parse(res); } catch(e) { json = {}; }
            
            const signUp = json.response || json;
            if (signUp && signUp.id) {
                log(`Sign Up Init (ID: ${signUp.id}). Waiting for email...`);
                
                // Poll IMAP
                let code = null;
                for(let i=0; i<12; i++) { // 60 seconds
                    await new Promise(r => setTimeout(r, 5000));
                    log(`Reading Inbox (${i+1}/12)...`);
                    try {
                        const val = await invoke("check_imap_code", { 
                           host: imapHost.value, 
                           port: Number(imapPort.value), 
                           user: imapUser.value, 
                           pass: imapPass.value 
                        });
                        if(val) {
                            code = val;
                            log("Code Fetched: " + code);
                            break;
                        }
                    } catch(e) {
                         // Ignore common errors during poll
                    }
                }
                
                if(code) {
                    log("Verifying Code...");
                    const vRes = await invoke("clerk_action_verify", { signUpId: signUp.id, code: code });
                    log("Final Result: " + vRes);
                } else {
                    log("Timeout: No verification code found in email.");
                }
            } else {
                log("Register Error: " + res.substring(0, 100));
            }
    } catch(e) {
        log("Error: " + e);
    } finally {
        autoRunning.value = false;
    }
}

async function loadAccounts() { try { accounts.value = await invoke("get_accounts"); activeAccountId.value = await invoke("get_active_id"); } catch (e) { console.error(e); } }
async function toggleListener(enable) {
  try { if (enable) { await invoke("uninject_orchids").catch(() => {}); const res = await invoke("start_listener"); statusMsg.value = res; isListening.value = true; } else { await invoke("stop_listener"); statusMsg.value = "Stopped"; isListening.value = false; } } catch (e) { isListening.value = false; }
}
async function importSession() { try { showNotification("Scanning..."); await invoke("import_current_session"); showNotification("Imported"); await loadAccounts(); } catch (e) { showNotification("Error: " + e); } }
async function setActive(id) { activeAccountId.value = id; try { await invoke("set_active_account", { id, capture: null }); showNotification("Switching..."); } catch (e) { showNotification("Error: " + e); } }
async function deleteAccount(id) { if (!confirm("Remove account?")) return; await invoke("delete_account", { id }); await loadAccounts(); }
async function addIdentity() { showLogoutModal.value = true; }
async function confirmLogout() { showLogoutModal.value = false; try { await invoke("logout_and_restart"); showNotification("Logged out"); } catch(e) {} }
async function toggleProxy() { try { if(!proxyRunning.value) { const res = await invoke("start_proxy", { port: Number(proxyPort.value) }); proxyStatus.value = res; proxyRunning.value = true; } else { await invoke("stop_proxy"); proxyStatus.value = "Stopped"; proxyRunning.value = false; } } catch(e) {} }
function showNotification(msg) { toastMsg.value = msg; showToast.value = true; setTimeout(() => (showToast.value = false), 3000); }

async function startGmailOAuth() {
    gmailOAuthLoading.value = true;
    log("Starting Gmail OAuth authorization...");
    log("A browser window will open. Please log in and authorize access.");
    try {
        const result = await invoke("gmail_oauth_start");
        log("OAuth Result: " + result);
        gmailOAuthStatus.value = "Authenticated";
        showNotification("Gmail authorized successfully!");
    } catch(e) {
        log("OAuth Error: " + e);
        showNotification("OAuth failed: " + e);
    } finally {
        gmailOAuthLoading.value = false;
    }
}

async function checkGmailOAuthStatus() {
    try {
        gmailOAuthStatus.value = await invoke("gmail_oauth_status");
    } catch(e) {
        gmailOAuthStatus.value = "Error";
    }
}

// Check OAuth status on load
checkGmailOAuthStatus();

</script>

<template>
  <div class="min-h-screen bg-background text-text-main font-sans selection:bg-primary/20">
    <Transition enter-active-class="transform transition duration-300 ease-out" enter-from-class="translate-y-full opacity-0" enter-to-class="translate-y-0 opacity-100" leave-active-class="transform transition duration-200 ease-in" leave-from-class="translate-y-0 opacity-100" leave-to-class="translate-y-full opacity-0">
      <div v-if="showToast" class="fixed bottom-6 right-6 z-50 bg-white border border-border text-text-main px-4 py-3 rounded-lg shadow-xl flex items-center gap-3">
        <div class="w-2 h-2 rounded-full bg-primary animate-pulse"></div>
        <span class="text-sm font-medium">{{ toastMsg }}</span>
      </div>
    </Transition>
    <div v-if="showLogoutModal" class="fixed inset-0 z-50 flex items-center justify-center bg-black/20 backdrop-blur-sm">
        <div class="bg-white rounded-xl p-6 w-[400px] shadow-2xl">
            <h3 class="text-lg font-bold mb-2">Add New Identity?</h3>
            <p class="text-sm text-text-sub mb-6">This will close Orchids and clear the current session.</p>
            <div class="flex justify-end gap-3">
                <button @click="showLogoutModal = false" class="px-4 py-2 rounded-lg text-sm font-medium text-text-sub hover:bg-background">Cancel</button>
                <button @click="confirmLogout" class="px-4 py-2 rounded-lg text-sm font-medium bg-primary text-white hover:bg-primary-hover shadow-lg shadow-primary/30">Continue</button>
            </div>
        </div>
    </div>
    <div class="sticky top-0 z-40 bg-background/80 backdrop-blur-md border-b border-border/50 px-6 py-3 flex items-center justify-between">
        <div class="flex items-center gap-3">
            <h1 class="font-bold text-lg tracking-tight">Orchis</h1>
        </div>
        <nav class="bg-surface rounded-full shadow-sm border border-border p-1 flex items-center gap-1">
            <button v-for="tab in [{ id: 'dashboard', label: 'Dashboard' }, { id: 'accounts', label: 'All Accounts' }, { id: 'automation', label: 'Automation' }, { id: 'proxy', label: 'API Proxy' }, { id: 'settings', label: 'Settings' }]" 
            :key="tab.id" @click="activeTab = tab.id" 
            :class="['px-4 py-1.5 rounded-full text-sm font-medium transition-all', activeTab === tab.id ? 'bg-primary/10 text-primary' : 'text-text-sub hover:text-text-main hover:bg-background']">{{ tab.label }}</button>
        </nav>
        <div class="flex items-center gap-3">
            <button class="w-9 h-9 rounded-full bg-surface border border-border flex items-center justify-center text-text-sub hover:text-text-main hover:bg-background transition-colors hover:shadow-sm">
                <component :is="Icons.Sun" class="w-5 h-5" />
            </button>
            <button class="w-9 h-9 rounded-full bg-surface border border-border text-xs font-bold text-text-main hover:bg-background transition-colors hover:shadow-sm flex items-center justify-center relative">
                EN
            </button>
        </div>
    </div>
    <main class="max-w-7xl mx-auto p-6 space-y-6">
        <div v-if="activeTab === 'dashboard'" class="space-y-6 animate-fade-in">
            <div class="flex items-center justify-between">
                <div><h2 class="text-2xl font-bold flex items-center gap-2">Hello, {{ greetingName }} 👋</h2><p class="text-text-sub text-sm">Welcome back to your identity command center.</p></div>
                <div class="flex gap-3">
                    <button @click="addIdentity" class="px-4 py-2 bg-white border border-border rounded-lg text-sm font-medium shadow-sm hover:bg-background flex items-center gap-2 transition-colors"><component :is="Icons.Plus" class="w-4 h-4 text-text-sub" /> Add Account</button>
                    <button @click="importSession" class="px-4 py-2 bg-primary text-white rounded-lg text-sm font-medium shadow-lg shadow-primary/20 hover:bg-primary-hover flex items-center gap-2 transition-colors"><component :is="Icons.Download" class="w-4 h-4" /> Capture Session</button>
                </div>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                <div v-for="(stat, idx) in [
                    { icon: Icons.Users, bg: 'bg-blue-50', color: 'text-blue-500', val: accounts.length, label: 'Total Identities' },
                    { icon: Icons.Zap, bg: 'bg-emerald-50', color: 'text-emerald-500', val: formatCredits(activeAccount?.user_info?.credits), label: 'Current Balance', sub: 'Active Account', subColor: 'text-emerald-600' },
                    { icon: Icons.Globe, bg: 'bg-purple-50', color: 'text-purple-500', val: proxyStatus === 'Stopped' ? 'OFF' : 'ON', label: 'API Proxy Status' },
                    { icon: Icons.Alert, bg: 'bg-orange-50', color: 'text-orange-500', val: '0', label: 'Warnings', sub: 'System Running Smoothly' }
                ]" :key="idx" class="bg-surface p-5 rounded-xl border border-border shadow-card flex flex-col justify-between h-32 hover:shadow-card-hover transition-shadow">
                    <div :class="['w-8 h-8 rounded-lg flex items-center justify-center mb-2', stat.bg, stat.color]"><component :is="stat.icon" class="w-5 h-5" /></div>
                    <div><h3 class="text-3xl font-bold tracking-tight">{{ stat.val }}</h3><p class="text-xs text-text-sub mt-1">{{ stat.label }}</p><p v-if="stat.sub" :class="['text-[10px] font-medium mt-0.5', stat.subColor || 'text-text-sub']">{{ stat.sub }}</p></div>
                </div>
            </div>
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                <div class="bg-surface p-6 rounded-xl border border-border shadow-card h-full flex flex-col">
                    <h3 class="text-sm font-bold flex items-center gap-2 mb-6"><component :is="Icons.Check" class="w-4 h-4 text-emerald-500" /> Current Account</h3>
                    <div v-if="activeAccount" class="flex-1 flex flex-col justify-center">
                        <div class="flex items-center justify-between mb-4">
                            <div><h4 class="text-lg font-bold text-text-main truncate">{{ activeAccount.email }}</h4><p class="text-xs text-text-sub">{{ activeAccount.display_name }}</p></div>
                             <span v-if="activeAccount.user_info?.plan === 'PRO'" class="px-2 py-1 rounded bg-primary/10 text-primary border border-primary/20 text-[10px] font-bold">PRO</span>
                             <span v-else class="px-2 py-1 rounded bg-gray-100 text-gray-500 border border-gray-200 text-[10px] font-bold">FREE</span>
                        </div>
                        <div class="space-y-4 mb-8">
                            <div><div class="flex justify-between text-xs mb-1.5"><span class="font-medium text-text-sub">Orchids Credits</span><span class="font-bold text-emerald-600">{{ formatCredits(activeAccount.user_info?.credits) }}</span></div><div class="h-2 w-full bg-background rounded-full overflow-hidden border border-border"><div class="h-full bg-emerald-500 rounded-full" :style="{ width: Math.min((activeAccount.user_info?.credits || 0) / 500000 * 100, 100) + '%' }"></div></div></div>
                            <div><div class="flex justify-between text-xs mb-1.5"><span class="font-medium text-text-sub">Session Health</span><span class="font-bold text-primary">100%</span></div><div class="h-2 w-full bg-background rounded-full overflow-hidden border border-border"><div class="h-full bg-primary rounded-full" style="width: 100%"></div></div></div>
                        </div>
                        <button @click="showLogoutModal = true" class="w-full py-2.5 border border-border rounded-lg text-sm font-medium text-text-sub hover:text-text-main hover:bg-background transition-colors">Change Account</button>
                    </div>
                    <div v-else class="flex-1 flex items-center justify-center text-text-sub text-sm">No active session detected.</div>
                </div>
                <div class="bg-surface p-6 rounded-xl border border-border shadow-card h-full flex flex-col">
                    <h3 class="text-sm font-bold flex items-center gap-2 mb-6"><component :is="Icons.Star" class="w-4 h-4 text-primary" /> Recommended</h3>
                    <div v-if="recommendAccounts.length > 0" class="space-y-3 max-h-[320px] overflow-y-auto pr-2 custom-scrollbar">
                        <div v-for="account in recommendAccounts" :key="account.id" class="p-4 bg-background border border-border rounded-lg flex items-center justify-between hover:border-primary/30 transition-colors cursor-pointer group flex-shrink-0" @click="setActive(account.id)">
                             <div><p class="text-[10px] font-bold text-text-sub uppercase mb-0.5 tracking-wider">Best for Credits</p><p class="text-sm font-medium text-text-main group-hover:text-primary transition-colors">{{ account.email }}</p></div>
                             <div class="px-2 py-1 rounded bg-emerald-100 text-emerald-700 text-xs font-bold whitespace-nowrap">{{ formatCredits(account.user_info?.credits) }}</div>
                        </div>
                        <button @click="setActive(recommendAccounts[0].id)" class="w-full mt-4 py-2.5 bg-primary text-white rounded-lg text-sm font-bold shadow-lg shadow-primary/20 hover:bg-primary-hover transition-colors flex-shrink-0 sticky bottom-0 z-10">Switch to Best Account</button>
                    </div>
                    <div v-else class="flex-1 flex items-center justify-center text-text-sub text-sm">Add more accounts to see recommendations.</div>
                </div>
            </div>
            <button @click="activeTab = 'accounts'" class="w-full py-3 bg-white border border-border text-text-main font-medium rounded-xl hover:bg-background transition-colors flex items-center justify-center gap-2 shadow-sm">View All Accounts <span class="text-lg leading-none transform translate-y-px">→</span></button>
        </div>
        <div v-if="activeTab === 'accounts'" class="space-y-6 animate-fade-in">
             <div class="bg-surface border border-border rounded-xl shadow-sm overflow-hidden">
                 <div class="overflow-x-auto">
                    <table class="w-full text-left border-collapse">
                        <thead>
                            <tr class="bg-background border-b border-border text-xs uppercase text-text-sub font-semibold tracking-wider">
                                <th class="px-6 py-4">Identity</th>
                                <th class="px-6 py-4">Credits</th>
                                <th class="px-6 py-4">Plan</th>
                                <th class="px-6 py-4 text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-border">
                            <tr v-for="account in accounts" :key="account.id" @click="setActive(account.id)" class="group cursor-pointer hover:bg-background transition-colors">
                                <td class="px-6 py-4">
                                    <div class="flex items-center gap-3">
                                        <div class="w-9 h-9 rounded-full bg-background border border-border flex items-center justify-center overflow-hidden">
                                            <img :src="account.avatar_url || 'https://github.com/identicons/default.png'" class="w-full h-full object-cover">
                                        </div>
                                        <div>
                                            <div class="flex items-center gap-2">
                                                <h4 class="font-bold text-sm text-text-main">{{ account.display_name }}</h4>
                                                <span v-if="activeAccountId === account.id" class="px-1.5 py-0.5 rounded bg-emerald-100 text-emerald-700 text-[10px] font-bold">ACTIVE</span>
                                            </div>
                                            <p class="text-xs text-text-sub">{{ account.email }}</p>
                                        </div>
                                    </div>
                                </td>
                                <td class="px-6 py-4">
                                    <div class="flex items-center gap-3">
                                        <div class="w-24 h-1.5 bg-background rounded-full overflow-hidden border border-border">
                                            <div class="h-full bg-emerald-500 rounded-full" :style="{ width: Math.min((account.user_info?.credits || 0) / 500000 * 100, 100) + '%' }"></div>
                                        </div>
                                        <span class="text-xs font-mono font-medium text-text-sub">{{ formatCredits(account.user_info?.credits) }}</span>
                                    </div>
                                </td>
                                <td class="px-6 py-4">
                                    <span v-if="account.user_info?.plan === 'PRO'" class="px-2 py-1 rounded bg-primary/10 text-primary border border-primary/20 text-[10px] font-bold">PRO</span>
                                    <span v-else class="px-2 py-1 rounded bg-background border border-border text-text-sub text-[10px] font-bold">FREE</span>
                                </td>
                                <td class="px-6 py-4 text-right">
                                    <button @click.stop="deleteAccount(account.id)" class="p-2 text-text-sub hover:text-red-500 hover:bg-red-50 rounded-lg transition-colors" title="Remove Account">
                                        <component :is="Icons.Trash" class="w-4 h-4" />
                                    </button>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                 </div>
                 <div v-if="accounts.length === 0" class="p-12 text-center text-text-sub text-sm">No accounts found. Add one to get started.</div>
             </div>
        </div>

        <!-- IMAP Settings Modal -->
        <div v-if="showImapSettings" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm animate-fade-in" @click.self="showImapSettings = false">
            <div class="bg-surface w-full max-w-md rounded-2xl shadow-2xl border border-border overflow-hidden flex flex-col max-h-[80vh]">
                <div class="p-4 border-b border-border flex justify-between items-center bg-background/50">
                    <h3 class="font-bold text-lg flex items-center gap-2"><component :is="Icons.Settings" class="w-5 h-5 text-text-sub"/> Email Profiles</h3>
                    <button @click="showImapSettings = false" class="text-text-sub hover:text-text-main"><component :is="Icons.Close" class="w-5 h-5"/></button>
                </div>
                
                <div class="flex-1 overflow-y-auto p-4 space-y-4">
                    <!-- Add New / Edit Form -->
                    <div v-if="isEditingProfile" class="bg-background border border-border rounded-xl p-4 space-y-3">
                        <div class="flex justify-between items-center mb-2">
                            <h4 class="font-bold text-xs uppercase text-primary">{{ editingProfile.id ? 'Edit Profile' : 'New Profile' }}</h4>
                            <div class="flex gap-2" v-if="!editingProfile.id">
                                <button @click="applyPreset('gmail')" class="text-[10px] px-2 py-1 bg-gray-100 rounded hover:bg-gray-200 text-gray-600">Gmail</button>
                                <button @click="applyPreset('qq')" class="text-[10px] px-2 py-1 bg-gray-100 rounded hover:bg-gray-200 text-gray-600">QQ</button>
                                <button @click="applyPreset('163')" class="text-[10px] px-2 py-1 bg-gray-100 rounded hover:bg-gray-200 text-gray-600">163</button>
                            </div>
                        </div>
                        <div><label class="text-[10px] font-bold text-text-sub mb-1 block">PROFILE NAME</label><input v-model="editingProfile.name" class="w-full px-3 py-2 bg-white border border-border rounded-lg text-xs" placeholder="e.g. My Primary Gmail"></div>
                        <div class="flex gap-3">
                            <div class="flex-1"><label class="text-[10px] font-bold text-text-sub mb-1 block">HOST</label><input v-model="editingProfile.host" class="w-full px-3 py-2 bg-white border border-border rounded-lg text-xs"></div>
                            <div class="w-20"><label class="text-[10px] font-bold text-text-sub mb-1 block">PORT</label><input v-model="editingProfile.port" type="number" class="w-full px-3 py-2 bg-white border border-border rounded-lg text-xs"></div>
                        </div>
                        <div><label class="text-[10px] font-bold text-text-sub mb-1 block">EMAIL (USER)</label><input v-model="editingProfile.user" class="w-full px-3 py-2 bg-white border border-border rounded-lg text-xs"></div>
                        <div><label class="text-[10px] font-bold text-text-sub mb-1 block">PASSWORD / APP KEY</label><input v-model="editingProfile.pass" type="password" class="w-full px-3 py-2 bg-white border border-border rounded-lg text-xs"></div>
                        <div class="flex gap-2 pt-2">
                            <button @click="saveProfile" class="flex-1 py-2 bg-primary text-white rounded-lg text-xs font-bold hover:bg-primary-hover">Save Profile</button>
                            <button @click="cancelEdit" class="px-4 py-2 bg-gray-100 text-text-sub rounded-lg text-xs font-bold hover:bg-gray-200">Cancel</button>
                        </div>
                    </div>

                    <!-- Profile List -->
                    <div v-else class="space-y-3">
                         <button @click="startNewProfile" class="w-full py-3 border border-dashed border-border rounded-xl text-text-sub text-xs font-bold hover:bg-background hover:text-primary transition-colors flex items-center justify-center gap-2">
                            <component :is="Icons.Plus" class="w-4 h-4"/> Add New Profile
                         </button>
                         <div v-for="p in imapProfiles" :key="p.id" class="p-3 bg-white border border-border rounded-xl flex justify-between items-center hover:shadow-sm transition-shadow group">
                             <div @click="selectProfile(p)" class="flex-1 cursor-pointer">
                                 <div class="font-bold text-sm text-text-main">{{ p.name }}</div>
                                 <div class="text-[10px] text-text-sub">{{ p.user }} • {{ p.host }}</div>
                             </div>
                             <div class="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                                 <button @click="editProfile(p)" class="p-1.5 text-text-sub hover:text-primary hover:bg-primary/10 rounded"><component :is="Icons.Edit" class="w-4 h-4"/></button>
                                 <button @click="deleteProfile(p.id)" class="p-1.5 text-text-sub hover:text-red-500 hover:bg-red-50 rounded"><component :is="Icons.Trash" class="w-4 h-4"/></button>
                             </div>
                         </div>
                         <div v-if="imapProfiles.length === 0" class="text-center text-xs text-text-sub py-4">No profiles saved.</div>
                    </div>
                </div>
            </div>
        </div>

        <div v-if="activeTab === 'automation'" class="grid grid-cols-1 lg:grid-cols-2 gap-6 animate-fade-in h-[calc(100vh-140px)]">
            <div class="bg-surface p-6 rounded-xl border border-border shadow-card flex flex-col gap-6">
                <div><h2 class="text-lg font-bold">Bot Configuration</h2><p class="text-sm text-text-sub">Configure login/registration automation (via Background API).</p></div>
                <div class="space-y-4">
                    <div class="flex gap-4 p-1 bg-background border border-border rounded-lg">
                        <button @click="autoMode = 'login'" :class="['flex-1 py-2 text-sm font-medium rounded-md transition-all', autoMode === 'login' ? 'bg-white shadow-sm text-text-main' : 'text-text-sub hover:text-text-main']">Login</button>
                        <button @click="autoMode = 'register'" :class="['flex-1 py-2 text-sm font-medium rounded-md transition-all', autoMode === 'register' ? 'bg-white shadow-sm text-text-main' : 'text-text-sub hover:text-text-main']">Register</button>
                    </div>
                <!-- Automation Inputs -->
                    <div>
                        <div class="flex justify-between">
                            <label class="text-xs font-bold text-text-sub mb-1 block">Target Email</label>
                            <button @click="fillTestData" :disabled="emailSuffix !== '@huaan666.site'" :class="['text-[10px] font-bold transition-colors', emailSuffix === '@huaan666.site' ? 'text-primary hover:underline' : 'text-gray-300 cursor-not-allowed']">Autofill</button>
                        </div>
                        <div class="flex gap-2">
                             <input v-model="emailPrefix" @input="updateAutoEmail" type="text" class="flex-1 px-3 py-2 bg-background border border-border rounded-lg text-sm" placeholder="username">
                             <select v-model="emailSuffix" @change="updateAutoEmail" class="w-40 px-3 py-2 bg-background border border-border rounded-lg text-sm text-text-main cursor-pointer appearance-none">
                                 <option value="@huaan666.site">@huaan666.site</option>
                                 <option value="@qq.com">@qq.com</option>
                                 <option value="@gmail.com">@gmail.com</option>
                                 <option value="@custom">Custom...</option>
                             </select>
                        </div>
                        <div v-if="isCustomSuffix" class="mt-2">
                             <input v-model="customSuffix" @input="updateAutoEmail" type="text" class="w-full px-3 py-2 bg-background border border-border rounded-lg text-sm" placeholder="@yourdomain.com">
                        </div>
                        <!-- Hidden full email for logic compatibility if needed, or we just rely on autoEmail being updated -->
                    </div>
                    <div><label class="text-xs font-bold text-text-sub mb-1 block">Target Password</label><input v-model="autoPass" type="password" class="w-full px-3 py-2 bg-background border border-border rounded-lg text-sm" placeholder="Password for account"></div>

                    <!-- IMAP SECTION -->
                    <div class="pt-4 border-t border-border mt-4">
                        <div class="flex items-center justify-between mb-3">
                            <h3 class="text-xs font-bold text-text-sub uppercase flex items-center gap-2"><component :is="Icons.Globe" class="w-3 h-3"/> IMAP Configuration</h3>
                             <button @click="showImapSettings = true" class="text-[10px] bg-primary/10 text-primary px-2 py-1 rounded hover:bg-primary/20 font-bold flex items-center gap-1">
                                <component :is="Icons.Settings" class="w-3 h-3"/> Manage Profiles
                             </button>
                        </div>
                        
                        <!-- Quick Profile Selector -->
                        <div class="mb-3">
                            <select v-model="selectedProfileId" @change="applyProfileById($event.target.value)" class="w-full px-3 py-2 bg-background border border-border rounded-lg text-xs text-text-main font-medium cursor-pointer">
                                <option value="" disabled selected>Select an Email Profile...</option>
                                <option v-for="p in imapProfiles" :key="p.id" :value="p.id">{{ p.name }} ({{ p.user }})</option>
                            </select>
                            <div v-if="imapProfiles.length === 0" class="mt-2 text-center">
                                <span class="text-[10px] text-text-sub">No profiles found. </span>
                                <button @click="showImapSettings = true" class="text-[10px] text-primary font-bold hover:underline">Create One</button>
                            </div>
                        </div>

                        <!-- Active Config Summary -->
                        <div v-if="imapHost" class="p-3 bg-background border border-border rounded-lg flex items-center gap-3">
                            <div class="w-8 h-8 rounded-full bg-surface border border-border flex items-center justify-center text-text-sub"><component :is="Icons.Check" class="w-4 h-4 text-emerald-500"/></div>
                            <div>
                                <div class="text-[10px] font-bold text-text-sub uppercase">Ready to Check</div>
                                <div class="text-xs font-medium text-text-main truncate max-w-[200px]">{{ imapUser || 'No User Set' }}</div>
                                <div class="text-[10px] text-text-sub">{{ imapHost }}:{{ imapPort }}</div>
                            </div>
                        </div>
                        
                        <!-- Gmail OAuth Button -->
                        <div class="mt-3 p-3 bg-gradient-to-r from-red-50 to-orange-50 border border-red-200 rounded-lg">
                            <div class="flex items-center justify-between">
                                <div>
                                    <div class="text-xs font-bold text-red-700">Gmail OAuth 2.0</div>
                                    <div class="text-[10px] text-red-600">{{ gmailOAuthStatus }}</div>
                                </div>
                                <button @click="startGmailOAuth" :disabled="gmailOAuthLoading" class="px-3 py-1.5 bg-red-500 hover:bg-red-600 text-white text-xs font-bold rounded-lg shadow transition-colors disabled:opacity-50">
                                    {{ gmailOAuthLoading ? 'Authorizing...' : (gmailOAuthStatus === 'Authenticated' ? '✓ Connected' : 'Authorize Gmail') }}
                                </button>
                            </div>
                        </div>
                    </div>
                
                    <div v-show="autoMode === 'register'" id="turnstile-widget" class="my-2 min-h-[65px] flex justify-center"></div>
                </div>
                <div class="mt-auto">
                    <button @click="startAutomation" :disabled="autoRunning" :class="['w-full py-3 rounded-lg text-white font-bold shadow-lg transition-all flex items-center justify-center gap-2', autoRunning ? 'bg-gray-400 cursor-not-allowed' : 'bg-primary hover:bg-primary-hover']">
                        <component :is="autoRunning ? Icons.Stop : Icons.Play" class="w-4 h-4" /> {{ autoRunning ? 'Running...' : 'Start Job' }}
                    </button>
                </div>
            </div>
            
            <!-- Console Output Panel -->
            <div class="bg-black/90 p-6 rounded-xl border border-gray-800 shadow-card flex flex-col font-mono text-xs">
                <div class="flex items-center justify-between mb-4 border-b border-gray-800 pb-2">
                    <h3 class="font-bold text-gray-400 flex items-center gap-2"><component :is="Icons.Terminal" class="w-4 h-4"/> Console Output</h3>
                    <button @click="autoLogs = []" class="text-gray-600 hover:text-gray-400">Clear</button>
                </div>
                <div class="flex-1 overflow-y-auto space-y-1 custom-scrollbar pr-2">
                    <div v-for="(log, i) in autoLogs" :key="i" class="text-emerald-500">{{ log }}</div>
                    <div v-if="autoLogs.length === 0" class="text-gray-700 italic">Ready for tasks...</div>
                </div>
            </div>
        </div>
        <div v-if="activeTab === 'proxy'" class="max-w-2xl mx-auto bg-surface p-8 rounded-xl border border-border shadow-card animate-fade-in"><h2 class="text-xl font-bold mb-6">API Proxy Configuration</h2><div class="space-y-4"><input v-model="proxyPort" type="number" class="w-full border border-border rounded-lg px-4 py-2" placeholder="Port" :disabled="proxyRunning"><button @click="toggleProxy" :class="['w-full py-2.5 font-bold rounded-lg text-white shadow-lg transition-colors', proxyRunning ? 'bg-red-500 hover:bg-red-600' : 'bg-primary hover:bg-primary-hover']">{{ proxyRunning ? 'Stop Server' : 'Start Server' }}</button></div></div>
        <div v-if="activeTab === 'settings'" class="max-w-2xl mx-auto bg-surface p-8 rounded-xl border border-border shadow-card animate-fade-in"><h2 class="text-xl font-bold mb-6">Settings</h2><p class="text-sm text-text-sub">Version 0.1.0 dev</p></div>
    </main>
  </div>
</template>
