<script setup>
import { ref, onMounted, computed, nextTick, watch } from "vue";
import { safeInvoke as invoke, safeListen as listen } from "./lib/tauri";

// --- State ---
const accounts = ref([]);
const activeTab = ref(localStorage.getItem('active_tab') || "dashboard");

watch(activeTab, (newTab) => {
    localStorage.setItem('active_tab', newTab);
});

const isListening = ref(false);
const statusMsg = ref("就绪");
const showToast = ref(false);
const toastMsg = ref("");
const debugLogs = ref([]);
const activeAccountId = ref(null);
const showLogoutModal = ref(false);

const proxyRunning = ref(false);
const proxyPort = ref(3000);
const proxyStatus = ref("已停止");
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
const gmailOAuthStatus = ref("未认证");
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

  listen("account-captured", (event) => { showNotification(`已捕获: ${event.payload.display_name}`); loadAccounts(); });
  listen("debug-log", (event) => {
    debugLogs.value.unshift(`[${new Date().toLocaleTimeString()}] ${event.payload}`);
    if (debugLogs.value.length > 200) debugLogs.value.pop();
  });
  listen("imap_log", (event) => {
    const now = new Date().toLocaleTimeString();
    autoLogs.value.unshift(`[${now}] ${event.payload}`);
    if (autoLogs.value.length > 100) autoLogs.value.pop();
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
                showNotification(`积分不足 (${formatCredits(credits)})。正在切换至 ${best.display_name}...`);
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

// Added totalCredits computed
const totalCredits = computed(() => {
    return accounts.value.reduce((acc, curr) => acc + (curr.user_info?.credits || 0), 0);
});

const greetingName = computed(() => {
    if (!activeAccount.value || !activeAccount.value.email) return "指挥官";
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
});

const emailPrefix = ref("");
const emailSuffix = ref("@gmail.com");

// Email Domain Management
const emailDomains = ref(JSON.parse(localStorage.getItem('email_domains') || '["@gmail.com"]'));
const showDomainManager = ref(false);
const newDomain = ref("");

function saveEmailDomains() {
    localStorage.setItem('email_domains', JSON.stringify(emailDomains.value));
}

function addEmailDomain() {
    let domain = newDomain.value.trim();
    if (!domain) return;
    if (!domain.startsWith('@')) domain = '@' + domain;
    if (!emailDomains.value.includes(domain)) {
        emailDomains.value.push(domain);
        saveEmailDomains();
    }
    newDomain.value = '';
}

function removeEmailDomain(domain) {
    if (domain === '@gmail.com') return; // Can't remove default
    emailDomains.value = emailDomains.value.filter(d => d !== domain);
    if (emailSuffix.value === domain) emailSuffix.value = '@gmail.com';
    saveEmailDomains();
}

function updateAutoEmail() {
    autoEmail.value = emailPrefix.value + emailSuffix.value;
}

function fillTestData() {
    // Only works for custom domains (not gmail)
    if (emailSuffix.value === '@gmail.com') return;
   
    // Username: letters and numbers only (no underscores)
    const rand = Math.random().toString(36).substring(2, 8);
    emailPrefix.value = `user${rand}`;
   
    updateAutoEmail();
   
    // Password: letters, numbers, @, .
    autoPass.value = `Pass.2026@${rand}`; 
}

watch(autoMode, (newMode) => {
    if (newMode === 'register' && !emailPrefix.value && emailSuffix.value !== '@gmail.com') {
        fillTestData();
    }
});

function applyPreset(type) {
    if(type === 'gmail') {
        editingProfile.value.host = 'imap.gmail.com';
        editingProfile.value.port = 993;
        editingProfile.value.name = 'Gmail 配置';
    } else if(type === 'qq') {
        editingProfile.value.host = 'imap.qq.com';
        editingProfile.value.port = 993;
        editingProfile.value.name = 'QQ 配置';
    } else if(type === '163') {
        editingProfile.value.host = 'imap.163.com';
        editingProfile.value.port = 993;
        editingProfile.value.name = '163 配置';
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
    if(!confirm("删除此配置？")) return;
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
    log(`已应用 IMAP 配置: ${p.name}`);
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
                else reject("Turnstile 未加载");
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
                    reject("Turnstile 错误");
                },
            });
        } catch(e) { reject(e); }
    });
}

async function startAutomation() {
    if(autoRunning.value) return;
    autoRunning.value = true;
    log(`正在开始 ${autoMode.value} 流程，目标: ${autoEmail.value}...`);
    
    try {
        let res;
        if (autoMode.value === 'login') {
            log("正在发送登录请求...");
            res = await invoke("clerk_action_login", { email: autoEmail.value, pass: autoPass.value });
            log("成功: " + res);
        } else if (autoMode.value === 'register') {
             log("正在打开注册窗口...");
             res = await invoke("clerk_action_register_webview", {
                 email: autoEmail.value,
                 pass: autoPass.value,
                 imapHost: imapHost.value || "",
                 imapPort: Number(imapPort.value) || 993,
                 imapUser: imapUser.value || "",
                 imapPass: imapPass.value || ""
             });
             log(res);
             log("请在新窗口完成注册。");
             // Stop here, user interaction required in popup
             return; 
        } else {
            log("未知模式");
            return;
        }
            
            let json;
            try { json = JSON.parse(res); } catch(e) { json = {}; }
            
            const signUp = json.response || json;
            if (signUp && signUp.id) {
                log(`注册初始化 (ID: ${signUp.id})。等待邮件...`);
                
                // Poll IMAP
                let code = null;
                for(let i=0; i<12; i++) { // 60 seconds
                    await new Promise(r => setTimeout(r, 5000));
                    log(`正在读取收件箱 (${i+1}/12)...`);
                    try {
                        const val = await invoke("check_imap_code", { 
                           host: imapHost.value, 
                           port: Number(imapPort.value), 
                           user: imapUser.value, 
                           pass: imapPass.value 
                        });
                        if(val) {
                            code = val;
                            log("获取验证码: " + code);
                            break;
                        }
                    } catch(e) {
                         // Ignore common errors during poll
                    }
                }
                
                if(code) {
                    log("正在验证代码...");
                    const vRes = await invoke("clerk_action_verify", { signUpId: signUp.id, code: code });
                    log("最终结果: " + vRes);
                } else {
                    log("超时: 邮件中未找到验证码。");
                }
            } else {
                log("注册错误: " + res.substring(0, 100));
            }
    } catch(e) {
        log("错误: " + e);
    } finally {
        autoRunning.value = false;
    }
}

async function loadAccounts() { try { accounts.value = await invoke("get_accounts"); activeAccountId.value = await invoke("get_active_id"); } catch (e) { console.error(e); } }
async function toggleListener(enable) {
  try { if (enable) { await invoke("uninject_orchids").catch(() => {}); const res = await invoke("start_listener"); statusMsg.value = res; isListening.value = true; } else { await invoke("stop_listener"); statusMsg.value = "已停止"; isListening.value = false; } } catch (e) { isListening.value = false; }
}
async function importSession() { try { showNotification("扫描中..."); await invoke("import_current_session"); showNotification("已导入"); await loadAccounts(); } catch (e) { showNotification("错误: " + e); } }
async function setActive(id) { activeAccountId.value = id; try { await invoke("set_active_account", { id, capture: null }); showNotification("正在切换..."); } catch (e) { showNotification("错误: " + e); } }
async function deleteAccount(id) { if (!confirm("移除账号？")) return; await invoke("delete_account", { id }); await loadAccounts(); }
async function addIdentity() { showLogoutModal.value = true; }
async function confirmLogout() { showLogoutModal.value = false; try { await invoke("logout_and_restart"); showNotification("已退出"); } catch(e) {} }
async function toggleProxy() { try { if(!proxyRunning.value) { const res = await invoke("start_proxy", { port: Number(proxyPort.value) }); proxyStatus.value = res; proxyRunning.value = true; } else { await invoke("stop_proxy"); proxyStatus.value = "已停止"; proxyRunning.value = false; } } catch(e) {} }
function showNotification(msg) { toastMsg.value = msg; showToast.value = true; setTimeout(() => (showToast.value = false), 3000); }

async function startGmailOAuth() {
    gmailOAuthLoading.value = true;
    log("正在启动 Gmail OAuth 授权...");
    log("浏览器窗口将打开。请登录并授权访问。");
    try {
        const result = await invoke("gmail_oauth_start");
        log("OAuth 结果: " + result);
        gmailOAuthStatus.value = "已认证";
        showNotification("Gmail 授权成功！");
    } catch(e) {
        log("OAuth 错误: " + e);
        showNotification("OAuth 失败: " + e);
    } finally {
        gmailOAuthLoading.value = false;
    }
}

async function checkGmailOAuthStatus() {
    try {
        gmailOAuthStatus.value = await invoke("gmail_oauth_status");
    } catch(e) {
        gmailOAuthStatus.value = "错误";
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
            <h3 class="text-lg font-bold mb-2">添加新身份？</h3>
            <p class="text-sm text-text-sub mb-6">这将关闭 Orchids 并清除当前会话。</p>
            <div class="flex justify-end gap-3">
                <button @click="showLogoutModal = false" class="px-4 py-2 rounded-lg text-sm font-medium text-text-sub hover:bg-background">取消</button>
                <button @click="confirmLogout" class="px-4 py-2 rounded-lg text-sm font-medium bg-primary text-white hover:bg-primary-hover shadow-lg shadow-primary/30">继续</button>
            </div>
        </div>
    </div>
    <div class="sticky top-0 z-40 bg-background/80 backdrop-blur-md border-b border-border/50 px-6 py-3 flex items-center justify-between">
        <div class="flex items-center gap-3">
            <h1 class="font-bold text-lg tracking-tight">Orchis</h1>
        </div>
        <nav class="bg-surface rounded-full shadow-sm border border-border p-1 flex items-center gap-1">
            <button v-for="tab in [{ id: 'dashboard', label: '仪表盘' }, { id: 'accounts', label: '所有账号' }, { id: 'automation', label: '自动化' }, { id: 'proxy', label: 'API 代理' }, { id: 'settings', label: '设置' }]" 
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
                <div><h2 class="text-2xl font-bold flex items-center gap-2">你好, {{ greetingName }} 👋</h2><p class="text-text-sub text-sm">欢迎回到你的身份指挥中心。</p></div>
                <div class="flex gap-3">
                    <button @click="addIdentity" class="px-4 py-2 bg-white border border-border rounded-lg text-sm font-medium shadow-sm hover:bg-background flex items-center gap-2 transition-colors"><component :is="Icons.Plus" class="w-4 h-4 text-text-sub" /> 添加账号</button>
                    <button @click="importSession" class="px-4 py-2 bg-primary text-white rounded-lg text-sm font-medium shadow-lg shadow-primary/20 hover:bg-primary-hover flex items-center gap-2 transition-colors"><component :is="Icons.Download" class="w-4 h-4" /> 捕获会话</button>
                </div>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                <div v-for="(stat, idx) in [
                    { icon: Icons.Users, bg: 'bg-blue-50', color: 'text-blue-500', val: accounts.length, label: '身份总数' },
                    { icon: Icons.Zap, bg: 'bg-emerald-50', color: 'text-emerald-500', val: formatCredits(activeAccount?.user_info?.credits), label: '当前余额', sub: '当前账号', subColor: 'text-emerald-600' },
                    { icon: Icons.Globe, bg: 'bg-purple-50', color: 'text-purple-500', val: proxyStatus === '已停止' ? 'OFF' : 'ON', label: 'API 代理状态' },
                    { icon: Icons.Star, bg: 'bg-orange-50', color: 'text-orange-500', val: formatCredits(totalCredits), label: '总积分池', sub: '所有账号合计' }
                ]" :key="idx" class="bg-surface p-5 rounded-xl border border-border shadow-card flex flex-col justify-between h-32 hover:shadow-card-hover transition-shadow">
                    <div :class="['w-8 h-8 rounded-lg flex items-center justify-center mb-2', stat.bg, stat.color]"><component :is="stat.icon" class="w-5 h-5" /></div>
                    <div><h3 class="text-3xl font-bold tracking-tight">{{ stat.val }}</h3><p class="text-xs text-text-sub mt-1">{{ stat.label }}</p><p v-if="stat.sub" :class="['text-[10px] font-medium mt-0.5', stat.subColor || 'text-text-sub']">{{ stat.sub }}</p></div>
                </div>
            </div>
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                <div class="bg-surface p-6 rounded-xl border border-border shadow-card h-full flex flex-col">
                    <h3 class="text-sm font-bold flex items-center gap-2 mb-6"><component :is="Icons.Check" class="w-4 h-4 text-emerald-500" /> 当前账号</h3>
                    <div v-if="activeAccount" class="flex-1 flex flex-col justify-center">
                        <div class="flex items-center justify-between mb-4">
                            <div><h4 class="text-lg font-bold text-text-main truncate">{{ activeAccount.email }}</h4><p class="text-xs text-text-sub">{{ activeAccount.display_name }}</p></div>
                             <span v-if="activeAccount.user_info?.plan === 'PRO'" class="px-2 py-1 rounded bg-primary/10 text-primary border border-primary/20 text-[10px] font-bold">PRO</span>
                             <span v-else class="px-2 py-1 rounded bg-gray-100 text-gray-500 border border-gray-200 text-[10px] font-bold">FREE</span>
                        </div>
                        <div class="space-y-4 mb-8">
                            <div><div class="flex justify-between text-xs mb-1.5"><span class="font-medium text-text-sub">Orchids 积分</span><span class="font-bold text-emerald-600">{{ formatCredits(activeAccount.user_info?.credits) }}</span></div><div class="h-2 w-full bg-background rounded-full overflow-hidden border border-border"><div class="h-full bg-emerald-500 rounded-full" :style="{ width: Math.min((activeAccount.user_info?.credits || 0) / 500000 * 100, 100) + '%' }"></div></div></div>
                            <div><div class="flex justify-between text-xs mb-1.5"><span class="font-medium text-text-sub">会话健康度</span><span class="font-bold text-primary">100%</span></div><div class="h-2 w-full bg-background rounded-full overflow-hidden border border-border"><div class="h-full bg-primary rounded-full" style="width: 100%"></div></div></div>
                        </div>
                        <button @click="showLogoutModal = true" class="w-full py-2.5 border border-border rounded-lg text-sm font-medium text-text-sub hover:text-text-main hover:bg-background transition-colors">切换账号</button>
                    </div>
                    <div v-else class="flex-1 flex items-center justify-center text-text-sub text-sm">未检测到活跃会话。</div>
                </div>
                <div class="bg-surface p-6 rounded-xl border border-border shadow-card h-full flex flex-col">
                    <h3 class="text-sm font-bold flex items-center gap-2 mb-6"><component :is="Icons.Star" class="w-4 h-4 text-primary" /> 推荐账号</h3>
                    <div v-if="recommendAccounts.length > 0" class="space-y-3 max-h-[320px] overflow-y-auto pr-2 custom-scrollbar">
                        <div v-for="account in recommendAccounts" :key="account.id" class="p-4 bg-background border border-border rounded-lg flex items-center justify-between hover:border-primary/30 transition-colors cursor-pointer group flex-shrink-0" @click="setActive(account.id)">
                             <div><p class="text-[10px] font-bold text-text-sub uppercase mb-0.5 tracking-wider">最佳积分</p><p class="text-sm font-medium text-text-main group-hover:text-primary transition-colors">{{ account.email }}</p></div>
                             <div class="px-2 py-1 rounded bg-emerald-100 text-emerald-700 text-xs font-bold whitespace-nowrap">{{ formatCredits(account.user_info?.credits) }}</div>
                        </div>
                        <button @click="setActive(recommendAccounts[0].id)" class="w-full mt-4 py-2.5 bg-primary text-white rounded-lg text-sm font-bold shadow-lg shadow-primary/20 hover:bg-primary-hover transition-colors flex-shrink-0 sticky bottom-0 z-10">切换至最佳账号</button>
                    </div>
                    <div v-else class="flex-1 flex items-center justify-center text-text-sub text-sm">添加更多账号以查看推荐。</div>
                </div>
            </div>
            <button @click="activeTab = 'accounts'" class="w-full py-3 bg-white border border-border text-text-main font-medium rounded-xl hover:bg-background transition-colors flex items-center justify-center gap-2 shadow-sm">查看所有账号 <span class="text-lg leading-none transform translate-y-px">→</span></button>
        </div>
        <div v-if="activeTab === 'accounts'" class="space-y-6 animate-fade-in">
             <div class="bg-surface border border-border rounded-xl shadow-sm overflow-hidden">
                 <div class="overflow-x-auto">
                    <table class="w-full text-left border-collapse">
                        <thead>
                            <tr class="bg-background border-b border-border text-xs uppercase text-text-sub font-semibold tracking-wider">
                                <th class="px-6 py-4">身份</th>
                                <th class="px-6 py-4">积分</th>
                                <th class="px-6 py-4">计划</th>
                                <th class="px-6 py-4 text-right">操作</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-border">
                            <!-- Total Row -->
                            <tr class="bg-surface/50 font-bold text-text-main">
                                <td class="px-6 py-3 text-right text-xs uppercase tracking-wider text-text-sub">总计</td>
                                <td class="px-6 py-3 text-sm text-emerald-600">{{ formatCredits(totalCredits) }}</td>
                                <td class="px-6 py-3 text-center text-xs text-text-sub">{{ accounts.length }} 个账号</td>
                                <td class="px-6 py-3"></td>
                            </tr>
                            <tr v-for="account in accounts" :key="account.id" @click="setActive(account.id)" class="group cursor-pointer hover:bg-background transition-colors">
                                <td class="px-6 py-4">
                                    <div class="flex items-center gap-3">
                                        <div class="w-9 h-9 rounded-full bg-background border border-border flex items-center justify-center overflow-hidden">
                                            <img :src="account.avatar_url || 'https://github.com/identicons/default.png'" class="w-full h-full object-cover">
                                        </div>
                                        <div>
                                            <div class="flex items-center gap-2">
                                                <h4 class="font-bold text-sm text-text-main">{{ account.display_name }}</h4>
                                                <span v-if="activeAccountId === account.id" class="px-1.5 py-0.5 rounded bg-emerald-100 text-emerald-700 text-[10px] font-bold">活跃</span>
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
                 <div v-if="accounts.length === 0" class="p-12 text-center text-text-sub text-sm">未找到账号。请添加一个以开始。</div>
             </div>
        </div>

        <!-- IMAP Settings Modal -->
        <div v-if="showImapSettings" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm animate-fade-in" @click.self="showImapSettings = false">
            <div class="bg-surface w-full max-w-md rounded-2xl shadow-2xl border border-border overflow-hidden flex flex-col max-h-[80vh]">
                <div class="p-4 border-b border-border flex justify-between items-center bg-background/50">
                    <h3 class="font-bold text-lg flex items-center gap-2"><component :is="Icons.Settings" class="w-5 h-5 text-text-sub"/> 邮箱配置</h3>
                    <button @click="showImapSettings = false" class="text-text-sub hover:text-text-main"><component :is="Icons.Close" class="w-5 h-5"/></button>
                </div>
                
                <div class="flex-1 overflow-y-auto p-4 space-y-4">
                    <!-- Add New / Edit Form -->
                    <div v-if="isEditingProfile" class="bg-background border border-border rounded-xl p-4 space-y-3">
                        <div class="flex justify-between items-center mb-2">
                            <h4 class="font-bold text-xs uppercase text-primary">{{ editingProfile.id ? '编辑配置' : '新配置' }}</h4>
                            <div class="flex gap-2" v-if="!editingProfile.id">
                                <button @click="applyPreset('gmail')" class="text-[10px] px-2 py-1 bg-gray-100 rounded hover:bg-gray-200 text-gray-600">Gmail</button>
                                <button @click="applyPreset('qq')" class="text-[10px] px-2 py-1 bg-gray-100 rounded hover:bg-gray-200 text-gray-600">QQ</button>
                                <button @click="applyPreset('163')" class="text-[10px] px-2 py-1 bg-gray-100 rounded hover:bg-gray-200 text-gray-600">163</button>
                            </div>
                        </div>
                        <div><label class="text-[10px] font-bold text-text-sub mb-1 block">配置名称</label><input v-model="editingProfile.name" class="w-full px-3 py-2 bg-white border border-border rounded-lg text-xs" placeholder="例如：我的主力 Gmail"></div>
                        <div class="flex gap-3">
                            <div class="flex-1"><label class="text-[10px] font-bold text-text-sub mb-1 block">主机 (Host)</label><input v-model="editingProfile.host" class="w-full px-3 py-2 bg-white border border-border rounded-lg text-xs"></div>
                            <div class="w-20"><label class="text-[10px] font-bold text-text-sub mb-1 block">端口 (Port)</label><input v-model="editingProfile.port" type="number" class="w-full px-3 py-2 bg-white border border-border rounded-lg text-xs"></div>
                        </div>
                        <div><label class="text-[10px] font-bold text-text-sub mb-1 block">邮箱 (用户名)</label><input v-model="editingProfile.user" class="w-full px-3 py-2 bg-white border border-border rounded-lg text-xs"></div>
                        <div><label class="text-[10px] font-bold text-text-sub mb-1 block">密码 / 应用专用密码</label><input v-model="editingProfile.pass" type="password" class="w-full px-3 py-2 bg-white border border-border rounded-lg text-xs"></div>
                        <div class="flex gap-2 pt-2">
                            <button @click="saveProfile" class="flex-1 py-2 bg-primary text-white rounded-lg text-xs font-bold hover:bg-primary-hover">保存配置</button>
                            <button @click="cancelEdit" class="px-4 py-2 bg-gray-100 text-text-sub rounded-lg text-xs font-bold hover:bg-gray-200">取消</button>
                        </div>
                    </div>

                    <!-- Profile List -->
                    <div v-else class="space-y-3">
                         <button @click="startNewProfile" class="w-full py-3 border border-dashed border-border rounded-xl text-text-sub text-xs font-bold hover:bg-background hover:text-primary transition-colors flex items-center justify-center gap-2">
                            <component :is="Icons.Plus" class="w-4 h-4"/> 添加新配置
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

        <!-- Email Domain Manager Modal -->
        <div v-if="showDomainManager" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm animate-fade-in" @click.self="showDomainManager = false">
            <div class="bg-surface p-6 rounded-xl border border-border shadow-card w-full max-w-md">
                <div class="flex justify-between items-center mb-4">
                    <h3 class="text-lg font-bold">邮箱域名</h3>
                    <button @click="showDomainManager = false" class="text-text-sub hover:text-text-main"><component :is="Icons.Close" class="w-5 h-5"/></button>
                </div>
                <p class="text-sm text-text-sub mb-4">管理自定义邮箱域名。Gmail 是默认项，无法移除。</p>
                
                <!-- Add New Domain -->
                <div class="flex gap-2 mb-4">
                    <input v-model="newDomain" @keyup.enter="addEmailDomain" type="text" class="flex-1 px-3 py-2 bg-background border border-border rounded-lg text-sm" placeholder="@yourdomain.com">
                    <button @click="addEmailDomain" class="px-4 py-2 bg-primary text-white rounded-lg text-sm font-medium hover:bg-primary-dark">添加</button>
                </div>
                
                <!-- Domain List -->
                <div class="space-y-2 max-h-60 overflow-y-auto">
                    <div v-for="domain in emailDomains" :key="domain" class="flex items-center justify-between p-3 bg-background rounded-lg border border-border">
                        <span class="text-sm font-medium">{{ domain }}</span>
                        <button v-if="domain !== '@gmail.com'" @click="removeEmailDomain(domain)" class="text-red-500 hover:text-red-600">
                            <component :is="Icons.Trash" class="w-4 h-4"/>
                        </button>
                        <span v-else class="text-xs text-text-sub">默认</span>
                    </div>
                </div>
            </div>
        </div>

        <div v-if="activeTab === 'automation'" class="grid grid-cols-1 lg:grid-cols-2 gap-6 animate-fade-in h-[calc(100vh-140px)]">
            <div class="bg-surface p-6 rounded-xl border border-border shadow-card flex flex-col gap-6">
                <div><h2 class="text-lg font-bold">机器人配置</h2><p class="text-sm text-text-sub">配置登录/注册自动化 (通过后台 API)。</p></div>
                <div class="space-y-4">
                    <div class="flex gap-4 p-1 bg-background border border-border rounded-lg">
                        <button @click="autoMode = 'login'" :class="['flex-1 py-2 text-sm font-medium rounded-md transition-all', autoMode === 'login' ? 'bg-white shadow-sm text-text-main' : 'text-text-sub hover:text-text-main']">登录</button>
                        <button @click="autoMode = 'register'" :class="['flex-1 py-2 text-sm font-medium rounded-md transition-all', autoMode === 'register' ? 'bg-white shadow-sm text-text-main' : 'text-text-sub hover:text-text-main']">注册</button>
                    </div>
                <!-- Automation Inputs -->
                    <div>
                        <div class="flex justify-between items-center">
                            <label class="text-xs font-bold text-text-sub mb-1 block">目标邮箱</label>
                            <div class="flex gap-2">
                                <button @click="showDomainManager = true" class="text-[10px] font-bold text-primary hover:underline">管理域名</button>
                                <button @click="fillTestData" :disabled="emailSuffix === '@gmail.com'" :class="['text-[10px] font-bold transition-colors', emailSuffix !== '@gmail.com' ? 'text-primary hover:underline' : 'text-gray-300 cursor-not-allowed']">自动填充</button>
                            </div>
                        </div>
                        <div class="flex gap-2">
                             <input v-model="emailPrefix" @input="updateAutoEmail" type="text" class="flex-1 px-3 py-2 bg-background border border-border rounded-lg text-sm" placeholder="username">
                             <select v-model="emailSuffix" @change="updateAutoEmail" class="w-40 px-3 py-2 bg-background border border-border rounded-lg text-sm text-text-main cursor-pointer appearance-none">
                                 <option v-for="domain in emailDomains" :key="domain" :value="domain">{{ domain }}</option>
                             </select>
                        </div>
                    </div>
                    <div><label class="text-xs font-bold text-text-sub mb-1 block">目标密码</label><input v-model="autoPass" type="password" class="w-full px-3 py-2 bg-background border border-border rounded-lg text-sm" placeholder="账号密码"></div>

                    <!-- Gmail OAuth Section -->
                    <div class="pt-4 border-t border-border mt-4">
                        <div class="p-4 bg-gradient-to-r from-red-50 to-orange-50 border border-red-200 rounded-lg">
                            <div class="flex items-center justify-between">
                                <div>
                                    <div class="text-sm font-bold text-red-700">Gmail OAuth 2.0</div>
                                    <div class="text-xs text-red-600">{{ gmailOAuthStatus }}</div>
                                </div>
                                <button @click="startGmailOAuth" :disabled="gmailOAuthLoading" class="px-4 py-2 bg-red-500 hover:bg-red-600 text-white text-sm font-bold rounded-lg shadow transition-colors disabled:opacity-50">
                                    {{ gmailOAuthLoading ? '正在授权...' : (gmailOAuthStatus === '已认证' ? '✓ 已连接' : '授权 Gmail') }}
                                </button>
                            </div>
                        </div>
                    </div>
                
                    <div v-show="autoMode === 'register'" id="turnstile-widget" class="my-2 min-h-[65px] flex justify-center"></div>
                </div>
                <div class="mt-auto">
                    <button @click="startAutomation" :disabled="autoRunning" :class="['w-full py-3 rounded-lg text-white font-bold shadow-lg transition-all flex items-center justify-center gap-2', autoRunning ? 'bg-gray-400 cursor-not-allowed' : 'bg-primary hover:bg-primary-hover']">
                        <component :is="autoRunning ? Icons.Stop : Icons.Play" class="w-4 h-4" /> {{ autoRunning ? '运行中...' : '开始任务' }}
                    </button>
                </div>
            </div>
            
            <!-- Console Output Panel -->
            <div class="bg-black/90 p-6 rounded-xl border border-gray-800 shadow-card flex flex-col font-mono text-xs">
                <div class="flex items-center justify-between mb-4 border-b border-gray-800 pb-2">
                    <h3 class="font-bold text-gray-400 flex items-center gap-2"><component :is="Icons.Terminal" class="w-4 h-4"/> 控制台输出</h3>
                    <button @click="autoLogs = []" class="text-gray-600 hover:text-gray-400">清除</button>
                </div>
                <div class="flex-1 overflow-y-auto space-y-1 custom-scrollbar pr-2">
                    <div v-for="(log, i) in autoLogs" :key="i" class="text-emerald-500">{{ log }}</div>
                    <div v-if="autoLogs.length === 0" class="text-gray-700 italic">准备就绪...</div>
                </div>
            </div>
        </div>
        <div v-if="activeTab === 'proxy'" class="max-w-2xl mx-auto bg-surface p-8 rounded-xl border border-border shadow-card animate-fade-in"><h2 class="text-xl font-bold mb-6">API 代理配置</h2><div class="space-y-4"><input v-model="proxyPort" type="number" class="w-full border border-border rounded-lg px-4 py-2" placeholder="端口" :disabled="proxyRunning"><button @click="toggleProxy" :class="['w-full py-2.5 font-bold rounded-lg text-white shadow-lg transition-colors', proxyRunning ? 'bg-red-500 hover:bg-red-600' : 'bg-primary hover:bg-primary-hover']">{{ proxyRunning ? '停止服务器' : '启动服务器' }}</button></div></div>
        <div v-if="activeTab === 'settings'" class="max-w-2xl mx-auto bg-surface p-8 rounded-xl border border-border shadow-card animate-fade-in">
            <h2 class="text-xl font-bold mb-6">设置</h2>
            <p class="text-sm text-text-sub">版本 0.1.0 dev</p>
        </div>
    </main>
  </div>
</template>
