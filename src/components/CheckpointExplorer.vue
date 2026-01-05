<script setup>
import { ref, onMounted, watch } from "vue";
import { safeInvoke as invoke } from "../lib/tauri";

const projects = ref([]);
const selectedProjectId = ref(null);
const checkpoints = ref([]);
const selectedCheckpointId = ref(null);
const checkpointFiles = ref([]);
const loading = ref(false);
const error = ref(null);

const Icons = {
  History: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"></path><path d="M3 3v5h5"></path><path d="M12 7v5l4 2"></path></svg>` },
  File: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"></path><polyline points="14 2 14 8 20 8"></polyline></svg>` },
  Folder: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>` },
  RotateCcw: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"></path><path d="M3 3v5h5"></path></svg>` },
  ChevronRight: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"></polyline></svg>` },
  AlertCircle: { template: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>` },
};

onMounted(async () => {
  await loadProjects();
});

async function loadProjects() {
  loading.value = true;
  error.value = null;
  try {
    projects.value = await invoke("get_projects");
    if (projects.value.length > 0) {
      selectedProjectId.value = projects.value[0].project_id;
    }
  } catch (e) {
    error.value = "加载项目失败: " + e;
  } finally {
    loading.value = false;
  }
}

watch(selectedProjectId, async (newId) => {
  if (newId) {
    await loadCheckpoints(newId);
  } else {
    checkpoints.value = [];
  }
});

async function loadCheckpoints(projectId) {
  loading.value = true;
  error.value = null;
  try {
    checkpoints.value = await invoke("get_checkpoints", { projectId });
    selectedCheckpointId.value = null;
    checkpointFiles.value = [];
  } catch (e) {
    error.value = "加载检查点失败: " + e;
    checkpoints.value = [];
  } finally {
    loading.value = false;
  }
}

async function selectCheckpoint(checkpointId) {
  selectedCheckpointId.value = checkpointId;
  loading.value = true;
  try {
    checkpointFiles.value = await invoke("get_checkpoint_files", { 
      projectId: selectedProjectId.value, 
      checkpointId 
    });
  } catch (e) {
    error.value = "加载文件列表失败: " + e;
  } finally {
    loading.value = false;
  }
}

async function handleRollback(checkpointId) {
  if (!confirm("确定要将项目回滚到此版本吗？这可能会覆盖当前未保存的更改。")) return;
  
  loading.value = true;
  try {
    await invoke("rollback_checkpoint", { 
      projectId: selectedProjectId.value, 
      checkpointId 
    });
    alert("回滚成功！");
  } catch (e) {
    error.value = "回滚失败: " + e;
  } finally {
    loading.value = false;
  }
}

function formatDate(timestamp) {
  return new Date(timestamp).toLocaleString();
}

function getStatusColor(status) {
  switch (status) {
    case 'added': return 'text-emerald-500';
    case 'modified': return 'text-blue-500';
    case 'deleted': return 'text-red-500';
    default: return 'text-text-sub';
  }
}
</script>

<template>
  <div class="flex flex-col h-full bg-surface rounded-xl border border-border overflow-hidden animate-fade-in">
    <!-- Header -->
    <div class="p-4 border-b border-border bg-background/50 flex items-center justify-between">
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center text-primary">
          <component :is="Icons.History" />
        </div>
        <div>
          <h2 class="text-sm font-bold">检查点资源管理器</h2>
          <p class="text-[10px] text-text-sub uppercase tracking-wider">历史版本与回滚</p>
        </div>
      </div>
      
      <div class="flex items-center gap-2">
        <label class="text-xs text-text-sub">项目:</label>
        <select v-model="selectedProjectId" class="bg-white border border-border rounded-md px-2 py-1 text-xs outline-none focus:ring-1 focus:ring-primary/30">
          <option v-for="p in projects" :key="p.project_id" :value="p.project_id">
            {{ p.project_name }}
          </option>
        </select>
        <button @click="loadProjects" class="p-1.5 text-text-sub hover:text-primary transition-colors" title="刷新">
          <component :is="Icons.RotateCcw" class="w-3.5 h-3.5" />
        </button>
      </div>
    </div>

    <!-- Main Content -->
    <div class="flex-1 flex overflow-hidden">
      <!-- Left: Checkpoints List -->
      <div class="w-1/3 border-r border-border overflow-y-auto flex flex-col bg-background/20">
        <div v-if="loading && checkpoints.length === 0" class="p-8 text-center text-text-sub text-sm">
          加载中...
        </div>
        <div v-else-if="checkpoints.length === 0" class="p-8 text-center text-text-sub text-sm">
          未发现检查点。
        </div>
        <div v-for="cp in checkpoints" :key="cp.id" 
          @click="selectCheckpoint(cp.id)"
          :class="['p-4 border-b border-border cursor-pointer transition-colors hover:bg-white', 
                   selectedCheckpointId === cp.id ? 'bg-white border-l-4 border-l-primary' : '']">
          <div class="flex justify-between items-start mb-1">
            <span class="text-[10px] font-mono text-text-sub bg-background px-1.5 py-0.5 rounded">#{{ cp.id }}</span>
            <span class="text-[10px] text-text-sub">{{ formatDate(cp.created_at) }}</span>
          </div>
          <div class="text-xs font-medium text-text-main truncate mb-2">{{ cp.message_id }}</div>
          <div class="flex items-center justify-between">
            <span class="text-[10px] text-text-sub flex items-center gap-1">
              <component :is="Icons.File" class="w-3 h-3" /> {{ cp.file_count }} 个文件
            </span>
            <button @click.stop="handleRollback(cp.id)" 
              class="px-2 py-1 bg-primary/10 text-primary hover:bg-primary text-[10px] font-bold rounded transition-colors hover:text-white">
              回滚
            </button>
          </div>
        </div>
      </div>

      <!-- Right: File List -->
      <div class="flex-1 overflow-y-auto bg-white p-4">
        <div v-if="!selectedCheckpointId" class="h-full flex flex-col items-center justify-center text-text-sub">
          <component :is="Icons.ChevronRight" class="w-12 h-12 opacity-10 mb-2" />
          <p class="text-sm">选择一个检查点查看变更文件</p>
        </div>
        <div v-else-if="checkpointFiles.length === 0" class="h-full flex items-center justify-center text-text-sub text-sm">
          该检查点没有记录文件。
        </div>
        <div v-else class="space-y-1">
          <div class="flex items-center justify-between mb-4 border-b border-border pb-2">
            <h3 class="text-xs font-bold text-text-sub">变更文件 ({{ checkpointFiles.length }})</h3>
          </div>
          <div v-for="file in checkpointFiles" :key="file.id" 
            class="flex items-center justify-between p-2 rounded hover:bg-background transition-colors text-xs group">
            <div class="flex items-center gap-3 overflow-hidden">
              <component :is="Icons.File" class="w-3.5 h-3.5 text-text-sub flex-shrink-0" />
              <span class="truncate font-mono">{{ file.file_path }}</span>
            </div>
            <div :class="['text-[10px] font-bold uppercase flex-shrink-0 ml-4', getStatusColor(file.status)]">
              {{ file.status }}
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Error Bar -->
    <div v-if="error" class="bg-red-50 border-t border-red-100 p-2 flex items-center justify-center gap-2">
      <component :is="Icons.AlertCircle" class="w-4 h-4 text-red-500" />
      <span class="text-[10px] text-red-600 font-medium">{{ error }}</span>
      <button @click="error = null" class="ml-2 text-red-400 hover:text-red-600">
        <component :is="Icons.Close" class="w-3 h-3" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.animate-fade-in {
  animation: fadeIn 0.3s ease-out;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(5px); }
  to { opacity: 1; transform: translateY(0); }
}

.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #e2e8f0;
  border-radius: 10px;
}
</style>
