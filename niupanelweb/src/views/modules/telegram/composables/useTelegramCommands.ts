import { reactive, ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import * as telegramApi from "@/api/telegram";
import type {
  TelegramCommand,
  TelegramCommandRequest,
} from "@/api/telegram";

export type TelegramCommandForm = TelegramCommandRequest & { id: number };

const createCommandPayload = (form: TelegramCommandForm): TelegramCommandRequest => ({
  name: form.name,
  script: form.script,
});

export function useTelegramCommands() {
  const commandsList = ref<TelegramCommand[]>([]);
  const showCmdDialog = ref(false);
  const cmdForm = reactive<TelegramCommandForm>({
    id: 0,
    name: "",
    script: "",
  });

  const loadCommands = async () => {
    const res = await telegramApi.getCommands({ page: 1, limit: 100 });
    if (res.data) commandsList.value = res.data.items;
  };

  const openCmdDialog = (row?: TelegramCommand) => {
    if (row) {
      cmdForm.id = row.id;
      cmdForm.name = row.name;
      cmdForm.script = row.script;
    } else {
      cmdForm.id = 0;
      cmdForm.name = "";
      cmdForm.script = "";
    }
    showCmdDialog.value = true;
  };

  const saveCmd = async () => {
    try {
      const payload = createCommandPayload(cmdForm);
      if (cmdForm.id > 0) {
        await telegramApi.updateCommand(cmdForm.id, payload);
      } else {
        await telegramApi.createCommand(payload);
      }
      ElMessage.success("保存成功");
      showCmdDialog.value = false;
      await loadCommands();
    } catch {
      ElMessage.error("保存失败");
    }
  };

  const handleDeleteCmd = (row: TelegramCommand) => {
    ElMessageBox.confirm(`确认删除指令 /${row.name}？`, "提示", { type: "warning" })
      .then(async () => {
        await telegramApi.deleteCommand(row.id);
        ElMessage.success("删除成功");
        await loadCommands();
      })
      .catch(() => {});
  };

  return {
    cmdForm,
    commandsList,
    handleDeleteCmd,
    loadCommands,
    openCmdDialog,
    saveCmd,
    showCmdDialog,
  };
}
