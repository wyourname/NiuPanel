<template>
  <el-drawer
    v-model="visible"
    title="机器人配置"
    :size="isMobile ? '100%' : '420px'"
    direction="rtl"
  >
    <div class="space-y-5">
      <div class="flex items-center justify-between rounded-md border border-light p-4">
        <div>
          <div class="text-sm font-bold text-default">服务开关</div>
          <div class="text-[10px] text-muted">开启/关闭 Telegram 机器人</div>
        </div>
        <el-switch
          v-model="form.enabled"
          inline-prompt
          active-text="开"
          inactive-text="关"
        />
      </div>

      <section class="space-y-3">
        <div
          class="border-l-3 border-primary pl-2 text-[10px] font-bold text-muted"
        >
          连接设置
        </div>
        <el-form-item label="Token">
          <el-input
            v-model="form.token"
            type="password"
            show-password
            placeholder="123456789:ABC..."
          />
        </el-form-item>
        <el-form-item label="管理员 Chat ID">
          <el-input v-model="form.admin_chat_id" placeholder="12345678" />
        </el-form-item>
        <el-form-item label="API 地址">
          <el-input v-model="form.api_base_url" placeholder="https://api.telegram.org" />
        </el-form-item>
      </section>

      <section class="space-y-3">
        <div
          class="border-l-3 border-orange-500 pl-2 text-[10px] font-bold text-muted"
        >
          代理
        </div>
        <div class="space-y-3 rounded-md border border-light bg-base/40 p-3">
          <el-form-item label="SOCKS5/HTTP 代理">
            <el-input
              v-model="form.proxy_url"
              placeholder="socks5://127.0.0.1:1080"
              :disabled="form.cf_proxy_enabled"
            />
          </el-form-item>
          <div class="pt-2 border-t border-light">
            <div class="flex items-center justify-between mb-2">
              <span class="text-xs font-bold text-default">CF 隧道</span>
              <el-switch v-model="form.cf_proxy_enabled" size="small" />
            </div>
            <div v-if="form.cf_proxy_enabled" class="grid gap-2">
              <el-input v-model="form.cf_host" placeholder="域名" size="small" />
              <el-input
                v-model="form.cf_token"
                type="password"
                show-password
                placeholder="令牌"
                size="small"
              />
              <el-input v-model="form.cf_ip" placeholder="目标 IP" size="small" />
            </div>
          </div>
        </div>
      </section>

      <div class="pt-4 flex gap-2">
        <el-button
          type="primary"
          class="h-9 flex-1 !rounded-md font-bold !text-xs"
          :loading="saving"
          @click="$emit('save')"
        >
          保存配置
        </el-button>
        <el-button
          class="h-9 !rounded-md px-5 font-bold !text-xs"
          :loading="testing"
          @click="$emit('test')"
        >
          测试
        </el-button>
      </div>
    </div>
  </el-drawer>
</template>

<script setup lang="ts">
import type { TelegramBotConfig } from "@/api/telegram";

defineProps<{
  isMobile: boolean;
  saving: boolean;
  testing: boolean;
}>();

defineEmits<{
  (e: "save"): void;
  (e: "test"): void;
}>();

const visible = defineModel<boolean>("visible", { required: true });
const form = defineModel<TelegramBotConfig>("form", { required: true });
</script>
