import { computed, reactive, ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import * as telegramApi from "@/api/telegram";
import type {
  TelegramWorkflow,
  TelegramWorkflowActionType,
  TelegramWorkflowEventType,
  TelegramWorkflowRequest,
} from "@/api/telegram";

export type TelegramWorkflowForm = TelegramWorkflowRequest & { id: number };

const createWorkflowPayload = (form: TelegramWorkflowForm): TelegramWorkflowRequest => ({
  action_type: form.action_type,
  config_json: form.config_json,
  event_type: form.event_type,
});

export function useTelegramWorkflows() {
  const workflowsList = ref<TelegramWorkflow[]>([]);
  const showWfDialog = ref(false);
  const wfForm = reactive<TelegramWorkflowForm>({
    id: 0,
    event_type: "alert",
    action_type: "notify",
    config_json: "{}",
  });

  const eventTypeLabel = (val: TelegramWorkflowEventType) => {
    const map: Record<string, string> = {
      failed: "任务失败",
      success: "任务成功",
      timeout: "任务超时",
      alert: "系统警报",
      login: "登录通知",
      cron: "定时触发 (Cron)",
    };
    return map[val] || val;
  };

  const actionTypeLabel = (val: TelegramWorkflowActionType) => {
    const map: Record<string, string> = {
      notify: "TG 通知",
      webhook: "Webhook",
      retry: "重试任务",
      shell: "Shell 命令",
      approval: "请求人工审批 (交互式)",
    };
    return map[val] || val;
  };

  const actionConfigLabel = computed(() => {
    const map: Record<string, string> = {
      notify: "通知模板 (JSON)",
      webhook: "Webhook URL",
      retry: "重试配置 (JSON)",
      shell: "Shell 脚本",
      approval: "审批配置 (JSON)",
    };
    if (wfForm.event_type === "cron") return "Cron 调度配置 (JSON)";
    return map[wfForm.action_type] || "配置";
  });

  const actionConfigPlaceholder = computed(() => {
    if (wfForm.event_type === "cron") {
      return "{\"cron\": \"0 8 * * *\", \"script\": \"echo 'Good morning!'\"}";
    }
    const map: Record<string, string> = {
      notify: "{\"template\": \"任务 {{task_id}} 失败\"}",
      webhook: "https://example.com/webhook",
      retry: "{\"max_retries\": 3, \"delay_seconds\": 60}",
      shell: "systemctl restart nginx",
      approval: "{\"message\": \"警报：是否清理磁盘？\\n[目前剩余1GB]\", \"script\": \"apt clean\"}",
    };
    return map[wfForm.action_type] || "{}";
  });

  const loadWorkflows = async () => {
    const res = await telegramApi.getWorkflows({ page: 1, limit: 100 });
    if (res.data) workflowsList.value = res.data.items;
  };

  const openWfDialog = (row?: TelegramWorkflow) => {
    if (row) {
      wfForm.id = row.id;
      wfForm.event_type = row.event_type;
      wfForm.action_type = row.action_type;
      wfForm.config_json = row.config_json;
    } else {
      wfForm.id = 0;
      wfForm.event_type = "alert";
      wfForm.action_type = "notify";
      wfForm.config_json = "{}";
    }
    showWfDialog.value = true;
  };

  const saveWf = async () => {
    try {
      const payload = createWorkflowPayload(wfForm);
      if (wfForm.id > 0) {
        await telegramApi.updateWorkflow(wfForm.id, payload);
      } else {
        await telegramApi.createWorkflow(payload);
      }
      ElMessage.success("保存成功");
      showWfDialog.value = false;
      await loadWorkflows();
    } catch {
      ElMessage.error("保存失败");
    }
  };

  const handleDeleteWf = (row: TelegramWorkflow) => {
    ElMessageBox.confirm("确认删除该自动化？", "提示", { type: "warning" })
      .then(async () => {
        await telegramApi.deleteWorkflow(row.id);
        ElMessage.success("删除成功");
        await loadWorkflows();
      })
      .catch(() => {});
  };

  return {
    actionConfigLabel,
    actionConfigPlaceholder,
    actionTypeLabel,
    eventTypeLabel,
    handleDeleteWf,
    loadWorkflows,
    openWfDialog,
    saveWf,
    showWfDialog,
    wfForm,
    workflowsList,
  };
}
