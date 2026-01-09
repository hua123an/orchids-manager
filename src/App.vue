<script setup>
import { ref, onMounted, computed, watch, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  LayoutDashboard,
  Users,
  Network,
  Settings,
  Plus,
  RefreshCw,
  Sun,
  ChevronRight,
  Download,
  CheckCircle2,
  AlertTriangle,
  Sparkles,
  Zap,
  Bot,
  Terminal,
  Play,
  StopCircle,
  Shield,
  CreditCard,
  Wallet,
  Coins,
} from "lucide-vue-next";

// --- State ---
const currentTab = ref(localStorage.getItem("ORCHIDS_LAST_TAB") || "dashboard"); // dashboard, automation, accounts, api, settings
watch(currentTab, (newTab) => {
  localStorage.setItem("ORCHIDS_LAST_TAB", newTab);
});
const accounts = ref([]);
const activeAccountId = ref(null);
const isLoading = ref(false);
const showAddAccountModal = ref(false);

// Automation State
const regEmail = ref("");
const regPass = ref("");
const selectedDomain = ref("gmail.com");
const newDomainInput = ref(""); // New Input State
const customizedDomains = ref([]);
const isGoogleAuthenticated = ref(false);
const autoLogs = ref([]);
const isAutoRunning = ref(false);

// --- Computed ---
const displayAccounts = computed(() => accounts.value);

const totalAccounts = computed(() => accounts.value.length);

const activeAccount = computed(() => {
  return accounts.value.find((a) => a.id === activeAccountId.value) || null;
});

// Real Data Aggregations
const totalCredits = computed(() => {
  return accounts.value.reduce(
    (sum, acc) => sum + (acc.user_info?.credits || 0),
    0
  );
});

const averageCredits = computed(() => {
  if (totalAccounts.value === 0) return 0;
  return Math.round(totalCredits.value / totalAccounts.value);
});

const lowBalanceCount = computed(() => {
  // Assuming < 50,000 is low
  return accounts.value.filter((a) => (a.user_info?.credits || 0) < 50000)
    .length;
});

const topAccounts = computed(() => {
  return [...accounts.value]
    .sort((a, b) => (b.user_info?.credits || 0) - (a.user_info?.credits || 0))
    .slice(0, 3);
});

const username = computed(() => "huaan jane");

// --- Lifecycle ---
onMounted(async () => {
  await init();
});

onUnmounted(() => {
  invoke("stop_listener").catch(() => {});
});

// --- Logic ---
async function init() {
  await invoke("ensure_patched").catch(() => {});
  await loadAccounts();
  await checkGoogleAuth();

  // Load Custom Domains
  const saved = localStorage.getItem("ORCHIDS_CUSTOM_DOMAINS");
  if (saved) {
    try {
      customizedDomains.value = JSON.parse(saved);
    } catch (e) {}
  }

  generateRandomUser();
  generateRandomPass();

  // Listeners
  listen("register_success", async (e) => {
    addLog(`✅ 注册成功: ${e.payload.email}`);
    await loadAccounts();
    notify("注册成功！");
    isAutoRunning.value = false;
  });

  listen("automation_log", (e) => {
    addLog(e.payload);
  });

  // Start Background Listener for Auto-Capture
  await invoke("start_listener").catch((e) =>
    console.error("Listener failed:", e)
  );
}

function addLog(msg) {
  const time = new Date().toLocaleTimeString("zh-CN", { hour12: false });
  autoLogs.value.unshift(`[${time}] ${msg}`);
  if (autoLogs.value.length > 200) autoLogs.value.pop();
}

async function loadAccounts() {
  try {
    accounts.value = await invoke("get_accounts");
    activeAccountId.value = await invoke("get_active_id");
  } catch (e) {
    console.error(e);
  }
}

async function refreshData() {
  isLoading.value = true;
  await loadAccounts();
  setTimeout(() => (isLoading.value = false), 800);
}

async function switchAccount(acc) {
  if (!acc) return;
  isLoading.value = true;
  try {
    await invoke("set_active_account", { id: acc.id, capture: null });
    activeAccountId.value = acc.id;
    notify(`已切换至 ${acc.display_name}`);
  } catch (e) {
    notify("切换失败");
  }
  isLoading.value = false;
}

// Custom Domain Logic
const displayedDomains = computed(() => {
  const defaults = ["gmail.com", "outlook.com", "hotmail.com", "yahoo.com"];
  // Merge defaults with custom domains, ensuring no duplicates
  const all = [...new Set([...defaults, ...customizedDomains.value])];
  return all;
});

const isCustomSuffix = computed(() => {
  return selectedDomain.value === "custom";
});

function handleDomainChange(e) {
  if (e.target.value === "custom") {
    // Keep internal state, just show input
  } else {
    newDomainInput.value = "";
  }
}

function updateAutoEmail() {
  // If "custom" isn't selected, logic is straightforward.
  // If custom logic is needed, we'd handle it elsewhere or via computed
}

// Add Custom Domain
function addCustomDomain() {
  const d = newDomainInput.value.trim();
  if (d && !displayedDomains.value.includes(d)) {
    customizedDomains.value.push(d);
    localStorage.setItem(
      "ORCHIDS_CUSTOM_DOMAINS",
      JSON.stringify(customizedDomains.value)
    );
    selectedDomain.value = d; // Auto select new
    newDomainInput.value = ""; // Clear input
    notify("Added custom domain: " + d);
  }
}

function removeDomain(d) {
  if (["gmail.com", "outlook.com", "hotmail.com", "yahoo.com"].includes(d))
    return;
  customizedDomains.value = customizedDomains.value.filter((x) => x !== d);
  localStorage.setItem(
    "ORCHIDS_CUSTOM_DOMAINS",
    JSON.stringify(customizedDomains.value)
  );
  selectedDomain.value = "gmail.com";
}

function generateRandomUser() {
  const chars = "abcdefghijklmnopqrstuvwxyz0123456789";
  let res = "";
  for (let i = 0; i < 10; i++) {
    res += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  regEmail.value = res;
}

function generateRandomPass() {
  const chars =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%";
  let res = "Orchids";
  for (let i = 0; i < 8; i++) {
    res += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  regPass.value = res;
}

async function checkGoogleAuth() {
  try {
    const status = await invoke("gmail_oauth_status");
    isGoogleAuthenticated.value = status === "Authenticated";
  } catch (e) {}
}

async function startGoogleAuth() {
  await invoke("gmail_oauth_start");
  // Poll for status or wait for event
  setTimeout(checkGoogleAuth, 5000);
}

// --- Automation ---

const manualCodeInput = ref("");
const showManualInput = ref(false);

// Listen for manual code request
listen("request_manual_code", () => {
  showManualInput.value = true;
  notify("自动获取失败，请手动输入验证码");
  // Ensure automation tab is focused?
});

async function submitManualCode() {
  if (manualCodeInput.value.length >= 4) {
    try {
      await invoke("set_manual_code", { code: manualCodeInput.value });
      addLog(`⌨️ 手动输入验证码: ${manualCodeInput.value}`);
      manualCodeInput.value = "";
      showManualInput.value = false;
    } catch (e) {
      notify("设置验证码失败: " + e);
    }
  }
}

async function runAuto() {
  if (isAutoRunning.value) {
    // Stop Logic?
    isAutoRunning.value = false;
    return;
  }

  isAutoRunning.value = true;
  showManualInput.value = false;
  manualCodeInput.value = "";
  autoLogs.value = [];
  addLog("🚀 启动全自动注册流程...");

  // Domain logic
  let dom = selectedDomain.value;
  if (dom === "custom") {
    dom = newDomainInput.value.trim();
  }
  if (!dom) dom = "gmail.com";

  // IMAP settings
  let host = "imap." + dom; // simplistic guess
  if (dom === "gmail.com") host = "imap.gmail.com";
  else if (dom === "outlook.com" || dom === "hotmail.com")
    host = "outlook.office365.com";
  else if (dom === "yahoo.com") host = "imap.mail.yahoo.com";
  // Custom? default to imap.domain

  // Password / OAuth Logic
  let passForImap = "password"; // Default fallback
  if (
    isGoogleAuthenticated.value &&
    !dom.includes("outlook") &&
    !dom.includes("hotmail")
  ) {
    passForImap = ""; // Empty password signals Backend to use OAuth
    addLog("🔑 检测到 Google 授权，将使用 OAuth 读取邮件");
  }

  try {
    await invoke("clerk_action_register_webview", {
      email: `${regEmail.value}@${dom}`,
      pass: regPass.value,
      imapHost: host,
      imapPort: 993,
      imapUser: `${regEmail.value}@${dom}`,
      imapPass: passForImap,
      capturePort: 8086,
    });
  } catch (e) {
    addLog(`❌ 错误: ${e}`);
    isAutoRunning.value = false;
  }
}

// --- Utils ---
const showToast = ref(false);
const toastMsg = ref("");
function notify(msg) {
  toastMsg.value = msg;
  showToast.value = true;
  setTimeout(() => (showToast.value = false), 3000);
}
</script>

<template>
  <div
    class="h-screen w-screen flex bg-[#F3F4F6] text-gray-800 font-sans dark:bg-gray-900 dark:text-gray-100 transition-colors duration-300"
  >
    <!-- Sidebar -->
    <aside
      class="w-64 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 flex flex-col shadow-sm"
    >
      <div
        class="p-6 flex items-center justify-center border-b border-gray-100 dark:border-gray-700"
      >
        <div
          class="w-10 h-10 bg-black dark:bg-white rounded-xl flex items-center justify-center mr-3 shadow-lg"
        >
          <Plus class="text-white dark:text-black w-6 h-6" />
        </div>
        <h1 class="text-xl font-bold tracking-tight">
          Orchids<span class="font-normal text-gray-500 text-sm block"
            >Manager</span
          >
        </h1>
      </div>

      <nav class="flex-1 p-4 space-y-1">
        <button
          v-for="item in [
            { id: 'dashboard', label: '仪表盘', icon: LayoutDashboard },
            { id: 'automation', label: '自动注册', icon: Zap },
            { id: 'accounts', label: '账号管理', icon: Users },
            { id: 'api', label: 'API 反代', icon: Network },
            { id: 'settings', label: '设置', icon: Settings },
          ]"
          :key="item.id"
          @click="currentTab = item.id"
          :class="[
            'w-full flex items-center px-4 py-3 rounded-xl transition-all duration-200 group',
            currentTab === item.id
              ? 'bg-black text-white shadow-md dark:bg-white dark:text-black'
              : 'hover:bg-gray-100 text-gray-600 dark:hover:bg-gray-700 dark:text-gray-400',
          ]"
        >
          <component
            :is="item.icon"
            :class="[
              'w-5 h-5 mr-3 transition-transform group-hover:scale-110',
              currentTab === item.id ? '' : 'text-gray-400',
            ]"
          />
          <span class="font-medium">{{ item.label }}</span>
          <ChevronRight
            v-if="currentTab === item.id"
            class="ml-auto w-4 h-4 opacity-50"
          />
        </button>
      </nav>

      <div class="p-4 border-t border-gray-200 dark:border-gray-700">
        <div
          class="bg-gradient-to-br from-indigo-500 to-purple-600 rounded-xl p-4 text-white shadow-lg relative overflow-hidden group cursor-pointer hover:shadow-xl transition-all"
          @click="currentTab = 'automation'"
        >
          <div
            class="absolute top-0 right-0 p-2 opacity-20 group-hover:opacity-30 transition-opacity"
          >
            <Sparkles class="w-16 h-16" />
          </div>
          <p
            class="text-xs font-medium uppercase tracking-wider opacity-80 mb-1"
          >
            Total Credits
          </p>
          <p class="text-2xl font-bold">{{ totalCredits.toLocaleString() }}</p>
          <div class="h-1 w-full bg-white/20 mt-3 rounded-full overflow-hidden">
            <div class="h-full bg-white w-2/3"></div>
          </div>
        </div>
      </div>
    </aside>

    <!-- Main Content -->
    <main class="flex-1 overflow-auto relative">
      <!-- Top Bar -->
      <header
        class="bg-white/80 dark:bg-gray-800/80 backdrop-blur-md sticky top-0 z-20 border-b border-gray-200 dark:border-gray-700 px-8 py-4 flex items-center justify-between"
      >
        <div>
          <!-- Dynamic Breadcrumb or Title -->
          <h2 class="text-2xl font-bold flex items-center">
            <span v-if="currentTab === 'dashboard'"
              >你好, {{ username }} 👋</span
            >
            <span v-else-if="currentTab === 'automation'"
              >Auto Registration</span
            >
            <span v-else-if="currentTab === 'accounts'">Accounts</span>
            <span v-else-if="currentTab === 'api'">API Proxy</span>
            <span v-else>Settings</span>
          </h2>
          <p
            v-if="currentTab === 'dashboard'"
            class="text-gray-500 text-sm mt-1"
          >
            欢迎回来，这是您的账号概览
          </p>
        </div>

        <div class="flex items-center space-x-3">
          <button
            @click="showAddAccountModal = true"
            class="flex items-center px-4 py-2 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 text-gray-700 text-sm font-medium transition shadow-sm dark:bg-gray-700 dark:border-gray-600 dark:text-gray-200"
          >
            <Plus class="w-4 h-4 mr-2" /> 添加账号
          </button>
          <button
            @click="refreshData"
            :disabled="isLoading"
            class="flex items-center px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg shadow-md transition-all active:scale-95 text-sm font-medium disabled:opacity-70 disabled:cursor-not-allowed"
          >
            <RefreshCw
              :class="['w-4 h-4 mr-2', isLoading ? 'animate-spin' : '']"
            />
            刷新数据
          </button>
          <button
            class="w-9 h-9 flex items-center justify-center rounded-full bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 transition"
          >
            <Sun class="w-5 h-5 text-gray-600 dark:text-yellow-400" />
          </button>
          <div
            class="w-9 h-9 rounded-full bg-gradient-to-r from-pink-500 to-orange-400 border-2 border-white shadow-sm"
          ></div>
        </div>
      </header>

      <div class="p-8 max-w-7xl mx-auto space-y-8 pb-32">
        <!-- DASHBOARD TAB -->
        <template v-if="currentTab === 'dashboard'">
          <!-- Stats Cards -->
          <div class="grid grid-cols-1 md:grid-cols-4 gap-6">
            <div
              class="bg-white dark:bg-gray-800 p-6 rounded-2xl shadow-sm border border-gray-100 dark:border-gray-700 flex flex-col justify-between hover:shadow-md transition-shadow"
            >
              <div
                class="w-10 h-10 rounded-full bg-blue-50 text-blue-600 flex items-center justify-center mb-4"
              >
                <Users class="w-5 h-5" />
              </div>
              <div>
                <h3 class="text-3xl font-bold text-gray-900 dark:text-white">
                  {{ totalAccounts }}
                </h3>
                <p class="text-gray-500 text-sm">总账号数</p>
              </div>
            </div>
            <div
              class="bg-white dark:bg-gray-800 p-6 rounded-2xl shadow-sm border border-gray-100 dark:border-gray-700 flex flex-col justify-between hover:shadow-md transition-shadow"
            >
              <div
                class="w-10 h-10 rounded-full bg-green-50 text-green-600 flex items-center justify-center mb-4"
              >
                <Wallet class="w-5 h-5" />
              </div>
              <div>
                <h3 class="text-3xl font-bold text-gray-900 dark:text-white">
                  {{ totalCredits.toLocaleString() }}
                </h3>
                <p class="text-gray-500 text-sm">总积分余额</p>
              </div>
            </div>
            <div
              class="bg-white dark:bg-gray-800 p-6 rounded-2xl shadow-sm border border-gray-100 dark:border-gray-700 flex flex-col justify-between hover:shadow-md transition-shadow"
            >
              <div
                class="w-10 h-10 rounded-full bg-purple-50 text-purple-600 flex items-center justify-center mb-4"
              >
                <Coins class="w-5 h-5" />
              </div>
              <div>
                <h3 class="text-3xl font-bold text-gray-900 dark:text-white">
                  {{ averageCredits.toLocaleString() }}
                </h3>
                <p class="text-gray-500 text-sm">平均积分</p>
              </div>
            </div>
            <div
              class="bg-white dark:bg-gray-800 p-6 rounded-2xl shadow-sm border border-gray-100 dark:border-gray-700 flex flex-col justify-between hover:shadow-md transition-shadow"
            >
              <div
                class="w-10 h-10 rounded-full bg-orange-50 text-orange-600 flex items-center justify-center mb-4"
              >
                <AlertTriangle class="w-5 h-5" />
              </div>
              <div>
                <h3 class="text-3xl font-bold text-gray-900 dark:text-white">
                  {{ lowBalanceCount }}
                </h3>
                <p class="text-gray-500 text-sm">低余额账号 (<5w)</p>
              </div>
            </div>
          </div>

          <!-- Active Account & Top Accounts -->
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
            <!-- Current Active -->
            <div
              class="bg-white dark:bg-gray-800 rounded-3xl shadow-sm border border-gray-200 dark:border-gray-700 p-8 relative overflow-hidden"
            >
              <div
                class="absolute top-0 right-0 w-32 h-32 bg-green-500/10 rounded-full blur-2xl -mr-10 -mt-10"
              ></div>

              <div
                class="flex items-center space-x-2 mb-6 text-green-600 font-semibold bg-green-50 w-fit px-3 py-1 rounded-full text-xs uppercase tracking-wider"
              >
                <CheckCircle2 class="w-4 h-4" /> <span>当前活跃账号</span>
              </div>

              <div
                v-if="activeAccount"
                class="flex flex-col h-full justify-between"
              >
                <div class="flex items-center space-x-5 mb-8">
                  <img
                    :src="
                      activeAccount.avatar_url ||
                      'https://api.dicebear.com/7.x/avataaars/svg?seed=' +
                        activeAccount.id
                    "
                    class="w-16 h-16 rounded-2xl bg-gray-100 border-2 border-white shadow-md"
                  />
                  <div>
                    <h4 class="text-xl font-bold">
                      {{ activeAccount.display_name }}
                    </h4>
                    <div
                      class="flex items-center text-gray-500 text-sm mt-1 space-x-2"
                    >
                      <span>{{ activeAccount.email }}</span>
                      <span class="w-1 h-1 bg-gray-300 rounded-full"></span>
                      <span class="font-mono text-xs opacity-70">{{
                        activeAccount.user_info?.plan || "FREE"
                      }}</span>
                    </div>
                  </div>
                  <div class="ml-auto">
                    <span
                      class="px-3 py-1 bg-blue-600 text-white text-xs font-bold rounded-full shadow-lg shadow-blue-200"
                      >ACTIVE</span
                    >
                  </div>
                </div>

                <div
                  class="bg-gray-50 dark:bg-gray-700/50 rounded-2xl p-6 mb-6"
                >
                  <div class="flex justify-between items-end mb-2">
                    <span
                      class="text-gray-500 font-medium text-xs uppercase tracking-wider"
                      >Available Credits</span
                    >
                    <span class="text-2xl font-black">{{
                      (activeAccount.user_info?.credits || 0).toLocaleString()
                    }}</span>
                  </div>
                  <div
                    class="w-full bg-gray-200 dark:bg-gray-600 rounded-full h-2 overflow-hidden"
                  >
                    <div
                      class="bg-blue-500 h-full rounded-full"
                      style="width: 75%"
                    ></div>
                  </div>
                </div>

                <button
                  class="w-full py-4 rounded-xl border border-gray-200 hover:border-gray-400 hover:bg-gray-50 transition font-medium text-gray-600"
                  @click="() => {
                    const others = accounts.filter(a => a.id !== activeAccount?.id && (a.user_info?.credits || 0) > 0).sort((a,b) => (b.user_info?.credits||0) - (a.user_info?.credits||0));
                    if(others.length > 0) {
                       switchAccount(others[0]);
                       notify(`已切换至高余额账号: ${others[0].display_name}`);
                    } else {
                       const anyOther = accounts.filter(a => a.id !== activeAccount?.id);
                       if (anyOther.length > 0) {
                          switchAccount(anyOther[0]);
                          notify(`已切换至账号: ${anyOther[0].display_name}`);
                       } else {
                          notify('没有其他可用账号');
                       }
                    }
                  }"
                >
                  切换账号 (Switch to Best)
                </button>
              </div>
              <div
                v-else
                class="flex flex-col items-center justify-center h-48 text-gray-400"
              >
                <Users class="w-12 h-12 mb-2 opacity-20" />
                <p>未选择活跃账号</p>
              </div>
            </div>

            <!-- Top Accounts List -->
            <div
              class="bg-white dark:bg-gray-800 rounded-3xl shadow-sm border border-gray-200 dark:border-gray-700 p-8"
            >
              <div
                class="flex items-center space-x-2 mb-6 text-yellow-600 font-semibold bg-yellow-50 w-fit px-3 py-1 rounded-full text-xs uppercase tracking-wider"
              >
                <Zap class="w-4 h-4" /> <span>高余额账号推荐</span>
              </div>

              <div class="space-y-4">
                <div
                  v-for="(acc, idx) in topAccounts"
                  :key="acc.id"
                  class="flex items-center p-4 rounded-xl border border-gray-100 hover:border-gray-300 hover:shadow-sm transition bg-white dark:bg-gray-700/30 cursor-pointer"
                  @click="switchAccount(acc)"
                >
                  <div
                    class="w-8 h-8 rounded-full bg-gray-100 flex items-center justify-center font-bold text-gray-400 text-xs mr-4"
                  >
                    #{{ idx + 1 }}
                  </div>
                  <div>
                    <p class="font-bold text-sm">{{ acc.display_name }}</p>
                    <p class="text-xs text-gray-400 truncate w-32">
                      {{ acc.email }}
                    </p>
                  </div>
                  <div class="ml-auto text-right">
                    <p class="font-black font-mono text-lg">
                      {{ (acc.user_info?.credits || 0).toLocaleString() }}
                    </p>
                    <p class="text-[10px] text-gray-400 uppercase">Credits</p>
                  </div>
                </div>
              </div>
              <button
                class="w-full text-center text-blue-600 font-medium text-sm mt-6 hover:underline flex items-center justify-center"
                @click="currentTab = 'accounts'"
              >
                查看更多排名 <ChevronRight class="w-4 h-4 ml-1" />
              </button>
            </div>
          </div>
        </template>

        <!-- AUTOMATION TAB -->
        <template v-if="currentTab === 'automation'">
          <div
            class="grid grid-cols-1 lg:grid-cols-2 gap-8 h-[calc(100vh-140px)]"
          >
            <!-- Control Panel -->
            <div
              class="bg-white dark:bg-gray-800 rounded-2xl shadow-sm border border-gray-200 dark:border-gray-700 flex flex-col overflow-hidden"
            >
              <div
                class="p-6 border-b border-gray-100 bg-gray-50 flex justify-between items-center"
              >
                <h3 class="font-bold flex items-center">
                  <Bot class="w-5 h-5 mr-2 text-indigo-600" /> Automation Config
                </h3>
                <div class="flex items-center space-x-2">
                  <div
                    class="flex items-center space-x-1 px-2 py-1 bg-white rounded-md border text-xs text-gray-500"
                  >
                    <span
                      class="w-2 h-2 rounded-full"
                      :class="
                        isGoogleAuthenticated ? 'bg-green-500' : 'bg-gray-300'
                      "
                    ></span>
                    <span>{{
                      isGoogleAuthenticated
                        ? "Google Auth Ready"
                        : "No Google Auth"
                    }}</span>
                  </div>
                </div>
              </div>

              <div class="p-6 space-y-6 flex-1 overflow-y-auto">
                <!-- Custom Domain Selector -->
                <div class="space-y-2">
                  <label class="text-sm font-medium text-gray-700"
                    >邮箱域名配置</label
                  >
                  <div class="flex space-x-2">
                    <select
                      v-model="selectedDomain"
                      @change="handleDomainChange"
                      class="flex-1 rounded-lg border-gray-300 bg-gray-50 p-2.5 text-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600"
                    >
                      <option v-for="d in displayedDomains" :key="d" :value="d">
                        {{ d }}
                      </option>
                      <option value="custom">Custom...</option>
                    </select>
                    <button
                      v-if="!isGoogleAuthenticated"
                      @click="startGoogleAuth"
                      class="px-3 py-2 bg-white border border-gray-300 rounded-lg text-xs hover:bg-gray-50 transition flex items-center whitespace-nowrap"
                    >
                      <img
                        src="https://www.svgrepo.com/show/475656/google-color.svg"
                        class="w-4 h-4 mr-2"
                      />
                      Link Google
                    </button>
                  </div>
                  <!-- Custom Input -->
                  <div
                    v-if="selectedDomain === 'custom'"
                    class="flex space-x-2 mt-2"
                  >
                    <input
                      v-model="newDomainInput"
                      placeholder="Enter custom domain (e.g. m.mysite.com)"
                      class="flex-1 rounded-lg border-gray-300 bg-gray-50 p-2 text-sm"
                    />
                    <button
                      @click="addCustomDomain"
                      class="bg-blue-600 text-white px-3 rounded-lg text-sm"
                    >
                      Add
                    </button>
                  </div>
                  <p class="text-xs text-gray-500">
                    使用 Gmail 授权可跳过密码配置，直接通过 API 读取验证码。
                  </p>
                </div>

                <div class="grid grid-cols-2 gap-4">
                  <div class="space-y-2">
                    <label class="text-sm font-medium text-gray-700"
                      >随机前缀</label
                    >
                    <div class="flex">
                      <input
                        v-model="regEmail"
                        class="flex-1 rounded-l-lg border-gray-300 bg-gray-50 p-2.5 text-sm font-mono"
                      />
                      <button
                        @click="generateRandomUser"
                        class="bg-gray-100 border border-l-0 border-gray-300 px-3 rounded-r-lg hover:bg-gray-200"
                      >
                        🎲
                      </button>
                    </div>
                  </div>
                  <div class="space-y-2">
                    <label class="text-sm font-medium text-gray-700"
                      >默认密码</label
                    >
                    <div class="flex">
                      <input
                        v-model="regPass"
                        class="flex-1 rounded-l-lg border-gray-300 bg-gray-50 p-2.5 text-sm font-mono"
                      />
                      <button
                        @click="generateRandomPass"
                        class="bg-gray-100 border border-l-0 border-gray-300 px-3 rounded-r-lg hover:bg-gray-200"
                      >
                        🎲
                      </button>
                    </div>
                  </div>
                </div>

                <!-- Manual Code Input (Conditional) -->
                <div
                  v-if="showManualInput"
                  class="bg-yellow-50 border border-yellow-200 rounded-xl p-4 animate-pulse"
                >
                  <h4 class="font-bold text-yellow-800 text-sm mb-2">
                    ⚠️ 需要手动输入验证码
                  </h4>
                  <div class="flex space-x-2">
                    <input
                      v-model="manualCodeInput"
                      placeholder="123456"
                      class="flex-1 text-center text-2xl font-mono tracking-widest rounded-lg border-yellow-400 focus:ring-yellow-500 p-2"
                      maxlength="8"
                    />
                    <button
                      @click="submitManualCode"
                      class="bg-yellow-600 text-white px-4 rounded-lg font-bold hover:bg-yellow-700"
                    >
                      提交
                    </button>
                  </div>
                </div>

                <div
                  class="bg-blue-50 dark:bg-blue-900/20 p-4 rounded-xl border border-blue-100 dark:border-blue-800"
                >
                  <h4
                    class="font-bold text-blue-800 dark:text-blue-300 text-sm mb-2"
                  >
                    Process Overview
                  </h4>
                  <ol
                    class="list-decimal list-inside text-xs space-y-1 text-blue-700 dark:text-blue-200"
                  >
                    <li>Reset Machine ID (UUID) & Clear Cache</li>
                    <li>Open Registration Webview</li>
                    <li>Inject JS to handle Clerk interaction</li>
                    <li>Wait for Email Code via IMAP/Gmail API</li>
                    <li>Auto-Copy Session Cookie to Manager</li>
                  </ol>
                </div>
              </div>

              <div class="p-6 border-t border-gray-100 bg-gray-50">
                <button
                  @click="runAuto"
                  :disabled="isAutoRunning"
                  :class="[
                    'w-full py-4 rounded-xl font-bold text-lg shadow-lg flex items-center justify-center transition-all active:scale-95',
                    isAutoRunning
                      ? 'bg-red-50 text-red-600 border border-red-200'
                      : 'bg-black text-white hover:bg-gray-800',
                  ]"
                >
                  <span v-if="isAutoRunning" class="flex items-center"
                    ><StopCircle class="w-6 h-6 mr-2" /> Stop / Running...</span
                  >
                  <span v-else class="flex items-center"
                    ><Play class="w-6 h-6 mr-2" /> Start Registration</span
                  >
                </button>
              </div>
            </div>

            <!-- Console / Logs -->
            <div
              class="bg-black rounded-2xl shadow-lg border border-gray-800 flex flex-col overflow-hidden font-mono text-sm text-green-400"
            >
              <div
                class="p-3 bg-gray-900 border-b border-gray-800 flex items-center justify-between"
              >
                <span class="flex items-center"
                  ><Terminal class="w-4 h-4 mr-2 text-gray-500" /> System
                  Logs</span
                >
                <div class="flex space-x-1.5">
                  <div class="w-3 h-3 rounded-full bg-red-500/50"></div>
                  <div class="w-3 h-3 rounded-full bg-yellow-500/50"></div>
                  <div class="w-3 h-3 rounded-full bg-green-500/50"></div>
                </div>
              </div>
              <div
                class="flex-1 p-4 overflow-y-auto space-y-1 scrollbar-thin scrollbar-thumb-gray-800"
              >
                <div
                  v-for="(log, i) in autoLogs"
                  :key="i"
                  class="opacity-90 hover:opacity-100 border-l-2 border-transparent hover:border-green-800 pl-2 transition-all"
                >
                  {{ log }}
                </div>
                <div
                  v-if="autoLogs.length === 0"
                  class="text-gray-600 italic mt-10 text-center"
                >
                  Ready for input...
                </div>
              </div>
            </div>
          </div>
        </template>

        <!-- ACCOUNTS LIST TEMPLATE (Placeholder for brevity) -->
        <template v-if="currentTab === 'accounts'">
          <div
            class="bg-white dark:bg-gray-800 rounded-2xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden"
          >
            <table class="w-full text-left text-sm">
              <thead
                class="bg-gray-50 dark:bg-gray-700/50 text-gray-500 uppercase font-medium"
              >
                <tr>
                  <th class="px-6 py-4">Status</th>
                  <th class="px-6 py-4">Account</th>
                  <th class="px-6 py-4">Credits</th>
                  <th class="px-6 py-4">Plan</th>
                  <th class="px-6 py-4">Actions</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-gray-100 dark:divide-gray-700">
                <tr
                  v-for="acc in accounts"
                  :key="acc.id"
                  class="hover:bg-gray-50 dark:hover:bg-gray-700/30 transition"
                >
                  <td class="px-6 py-4">
                    <div
                      class="w-2 h-2 rounded-full"
                      :class="
                        acc.id === activeAccountId
                          ? 'bg-green-500 box-shadow-green'
                          : 'bg-gray-300'
                      "
                    ></div>
                  </td>
                  <td class="px-6 py-4 font-medium flex items-center">
                    <img
                      :src="acc.avatar_url"
                      class="w-8 h-8 rounded-full bg-gray-100 mr-3"
                    />
                    <div>
                      <p>{{ acc.display_name }}</p>
                      <p class="text-xs text-gray-400">{{ acc.email }}</p>
                    </div>
                  </td>
                  <td class="px-6 py-4 font-mono font-bold">
                    {{ (acc.user_info?.credits || 0).toLocaleString() }}
                  </td>
                  <td class="px-6 py-4">
                    <span
                      class="px-2 py-1 rounded-md text-xs font-bold bg-gray-100 text-gray-600"
                      >{{ acc.user_info?.plan || "FREE" }}</span
                    >
                  </td>
                  <td class="px-6 py-4">
                    <button
                      v-if="acc.id !== activeAccountId"
                      @click="switchAccount(acc)"
                      class="text-blue-600 hover:underline"
                    >
                      Use
                    </button>
                    <span v-else class="text-gray-400 cursor-default"
                      >Active</span
                    >
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </template>
      </div>
    </main>

    <!-- Toast Notification -->
    <div
      v-if="showToast"
      class="fixed bottom-6 right-6 bg-gray-900 text-white px-6 py-3 rounded-xl shadow-2xl flex items-center space-x-3 animate-slide-up z-50"
    >
      <CheckCircle2 class="text-green-400 w-5 h-5" />
      <span>{{ toastMsg }}</span>
    </div>

    <!-- Modals (Add Account, etc) - Leaving placeholder -->
  </div>
</template>

<style>
@import url("https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;900&family=JetBrains+Mono:wght@400;500&display=swap");

:root {
  font-family: "Inter", sans-serif;
}

/* Custom Scrollbar */
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: #e5e7eb;
  border-radius: 99px;
}
::-webkit-scrollbar-thumb:hover {
  background: #d1d5db;
}

.box-shadow-green {
  box-shadow: 0 0 0 4px rgba(34, 197, 94, 0.2);
}

@keyframes slide-up {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
.animate-slide-up {
  animation: slide-up 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}
</style>
