<template>
  <div class="space-y-6">
    <el-form label-position="top" class="space-y-8">
      <section class="space-y-4">
        <div class="flex items-center justify-between gap-3 px-1">
          <div class="flex items-center gap-2">
            <div class="w-1.5 h-1.5 rounded-full bg-primary"></div>
            <h4 class="text-[13px] font-bold text-default">触发事件</h4>
          </div>
          <el-button
            type="primary"
            :loading="saving"
            @click="handleSave"
            class="!h-8 !rounded-md !px-3 !text-[12px] font-bold"
          >
            <div class="i-ep-check mr-1 text-sm"></div>
            保存修改
          </el-button>
        </div>

        <el-checkbox-group v-model="selectedEvents" class="flex flex-wrap gap-3">
          <el-checkbox value="failed" border class="!mr-0 !h-auto !rounded-md !px-4 !py-2.5 transition-colors">任务失败</el-checkbox>
          <el-checkbox value="success" border class="!mr-0 !h-auto !rounded-md !px-4 !py-2.5 transition-colors">任务成功</el-checkbox>
          <el-checkbox value="login" border class="!mr-0 !h-auto !rounded-md !px-4 !py-2.5 transition-colors">系统登录</el-checkbox>
        </el-checkbox-group>
      </section>

      <div class="grid grid-cols-1 lg:grid-cols-2 gap-x-12 gap-y-10">

        <section class="space-y-4">
          <div class="flex items-center justify-between px-1">
            <div class="flex items-center gap-2">
              <div class="w-1.5 h-1.5 rounded-full bg-orange-500"></div>
              <h4 class="text-[13px] font-bold text-default">Webhook 推送</h4>
            </div>
            <el-button type="primary" link @click="testNotify('webhook')" class="!h-auto !p-0 text-[11px] font-bold">
              <div class="i-ep-promotion mr-1 text-xs"></div>发送测试
            </el-button>
          </div>

          <el-form-item label-class="!hidden" class="!mb-0">
            <div class="space-y-2.5 w-full">
              <label class="label-xs ml-1">Webhook URL</label>
              <el-input
                v-model="form.webhook_url"
                type="textarea"
                :rows="4"
                placeholder="钉钉/飞书/企业微信/Telegram URL..."
                class="modern-input"
              />
              <p class="text-[10px] text-muted/50 leading-relaxed px-1">支持主流 IM 平台的 Webhook 地址。系统将通过 POST 请求发送 JSON 格式的通知载荷。</p>
            </div>
          </el-form-item>
        </section>

        <section class="space-y-4">
          <div class="flex items-center justify-between px-1">
            <div class="flex items-center gap-2">
              <div class="w-1.5 h-1.5 rounded-full bg-indigo-500"></div>
              <h4 class="text-[13px] font-bold text-default">邮件通知（SMTP）</h4>
            </div>
            <el-button type="primary" link @click="testNotify('mail')" class="!h-auto !p-0 text-[11px] font-bold">
              <div class="i-ep-promotion mr-1 text-xs"></div>发送测试
            </el-button>
          </div>

          <div class="space-y-4">
            <div class="grid grid-cols-1 gap-4 sm:grid-cols-12">
              <div class="space-y-2 sm:col-span-8">
                <label class="label-xs ml-1">SMTP 主机</label>
                <el-input v-model="form.mail_host_only" placeholder="smtp.gmail.com" class="modern-input" />
              </div>
              <div class="space-y-2 sm:col-span-4">
                <label class="label-xs ml-1">端口</label>
                <el-input v-model="form.mail_port" placeholder="465" class="modern-input" />
              </div>
            </div>

            <div class="space-y-2">
              <label class="label-xs ml-1">告警接收邮箱</label>
              <div class="flex gap-2">
                <el-input v-model="form.mail_to" placeholder="receiver@example.com" class="flex-1 modern-input" />
                <el-tooltip content="使用当前账户邮箱" placement="top">
                  <el-button @click="syncFromProfile" class="!h-9 !w-9 !rounded-md flex-center">
                    <div class="i-ep-user"></div>
                  </el-button>
                </el-tooltip>
              </div>
            </div>

            <div class="grid grid-cols-1 gap-4 pt-2 sm:grid-cols-2">
              <div class="space-y-2">
                <label class="label-xs ml-1">用户名</label>
                <el-input v-model="form.mail_username" placeholder="user@gmail.com" class="modern-input" />
              </div>
              <div class="space-y-2">
                <label class="label-xs ml-1">密码</label>
                <el-input v-model="form.mail_password" type="password" show-password class="modern-input" />
              </div>
            </div>
          </div>
        </section>
      </div>
    </el-form>
  </div>
</template>


<script setup lang="ts">
import { ref, reactive, onMounted, computed } from "vue";
import { ElMessage } from "element-plus";
import * as settingsApi from "../../../api/settings";
import { useUserStore } from "../../../stores/user";
import type {
  NotificationSettings,
  NotificationTestRequest,
  NotificationTestType,
  SettingItem,
} from "@/types";

const userStore = useUserStore();
const saving = ref(false);
type NotificationForm = NotificationSettings & {
  mail_host_only: string;
  mail_port: string;
};

const form = reactive<NotificationForm>({
  webhook_url: "",
  events: "failed,login",
  mail_host: "",
  mail_host_only: "",
  mail_port: "",
  mail_username: "",
  mail_password: "",
  mail_to: "",
});

const toSettingsMap = (items: SettingItem[]) =>
  new Map(items.map((item) => [item.key, item.value]));

const applyMailHost = (value: string) => {
  form.mail_host = value;
  const parts = value.split(":");
  if (parts.length > 1) {
    form.mail_port = parts.pop() || "";
    form.mail_host_only = parts.join(":");
  } else {
    form.mail_host_only = value;
    form.mail_port = "465";
  }
};

const selectedEvents = computed({
  get: () => form.events.split(",").filter(Boolean),
  set: (v: string[]) => (form.events = v.join(",")),
});

const load = async () => {
  try {
    const res = await settingsApi.getSettings();
    if (res.data) {
      const settings = toSettingsMap(res.data);
      form.webhook_url = settings.get("notify.webhook_url") || form.webhook_url;
      form.events = settings.get("notify.events") || form.events;
      const mailHost = settings.get("mail.host");
      if (mailHost !== undefined) applyMailHost(mailHost);
      form.mail_username = settings.get("mail.username") || form.mail_username;
      form.mail_password = settings.get("mail.password") || form.mail_password;
      form.mail_to = settings.get("mail.to") || form.mail_to;
    }
  } catch (e) {}
};

const syncFromProfile = async () => {
  if (!userStore.userInfo.email) {
    await userStore.fetchUserProfile();
  }
  if (userStore.userInfo.email) {
    form.mail_to = userStore.userInfo.email;
    ElMessage.success("已同步账户邮箱");
  } else {
    ElMessage.warning("您的账户尚未绑定邮箱");
  }
};

const getFullHost = () => {
  if (form.mail_port) return `${form.mail_host_only}:${form.mail_port}`;
  return form.mail_host_only;
};

const handleSave = async () => {
  saving.value = true;
  try {
    form.mail_host = getFullHost();
    await settingsApi.updateNotificationSettings({ ...form });
    ElMessage.success("通知配置已保存");
  } finally {
    saving.value = false;
  }
};

const testNotify = async (type: NotificationTestType) => {
  try {
    form.mail_host = getFullHost();
    const testData: NotificationTestRequest = {
      notify_type: type,
      "notify.webhook_url": form.webhook_url,
      "mail.host": form.mail_host,
      "mail.username": form.mail_username,
      "mail.password": form.mail_password,
      "mail.to": form.mail_to,
    };
    await settingsApi.testNotification(testData);
    ElMessage.success("测试消息已发送，请检查接收端");
  } catch (e) {}
};

onMounted(load);
</script>
