export const isTauri = () => {
  return typeof window !== 'undefined' && window.__TAURI_INTERNALS__ !== undefined;
};

export async function safeInvoke(command, args = {}) {
  if (isTauri()) {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      return await invoke(command, args);
    } catch (e) {
      console.error("Tauri invoke error:", e);
      return null;
    }
  }
  
  console.log(`[Mock] Invoke: ${command}`, args);
  
  // Mock responses for web preview
  const mocks = {
    'get_accounts': [
      { id: 'm1', display_name: 'Alpha User', email: 'alpha@example.com', user_info: { credits: 125000, plan: 'PRO' } },
      { id: 'm2', display_name: 'Beta Tester', email: 'beta@test.com', user_info: { credits: 8000, plan: 'FREE' } }
    ],
    'get_active_id': 'm1',
    'gmail_oauth_status': '已认证 (模拟)',
    'start_listener': '监听已启动 (模拟)',
    'stop_listener': '监听已停止 (模拟)',
    'refresh_active_account': { id: 'm1', display_name: 'Alpha User', email: 'alpha@example.com', user_info: { credits: 125000, plan: 'PRO' } }
  };
  
  return mocks[command] || null;
}

export async function safeListen(event, callback) {
  if (isTauri()) {
    try {
      const { listen } = await import("@tauri-apps/api/event");
      return await listen(event, callback);
    } catch (e) {
      console.error("Tauri listen error:", e);
      return () => {};
    }
  }
  
  console.log(`[Mock] Listen: ${event}`);
  return () => console.log(`[Mock] Unlisten: ${event}`);
}
