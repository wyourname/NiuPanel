<template>
  <div class="space-y-6">
    <div class="flex flex-col items-center justify-between gap-5 border-b border-light pb-5 md:flex-row">
      <div class="flex items-center gap-5">
        <div class="h-12 w-12 shrink-0 rounded-md bg-primary text-lg font-bold text-white flex-center">
          {{ userStore.userInfo.username?.charAt(0).toUpperCase() }}
        </div>
        <div class="flex flex-col">
          <div class="flex items-center gap-2.5">
            <span class="text-[16px] font-bold text-default">{{ userStore.userInfo.username }}</span>
            <span class="accent-subtle rounded px-2 py-0.5 text-[10px] font-bold">{{ userStore.userInfo.role }}</span>
          </div>
          <div class="text-[13px] text-muted mt-1.5 flex items-center gap-4">
            <span class="flex items-center gap-1.5 font-medium">
              <div class="i-ep-message opacity-60"></div>
              {{ userStore.userInfo.email || "未绑定邮箱" }}
            </span>
            <el-tag v-if="userStore.userInfo.email_verified" type="success" size="small" effect="plain" class="!h-5 !px-2 border-none !bg-success/10 !text-success font-bold text-[10px]">已验证</el-tag>
          </div>
        </div>
      </div>
      <div class="flex flex-col items-end gap-3">
        <div class="flex flex-col items-end text-[11px] text-muted/60 gap-1.5 font-mono">
          <div v-if="userStore.userInfo.last_login_at" class="flex items-center gap-2">
            <div class="i-ep-clock text-xs"></div> 上次登录：{{ formatDate(userStore.userInfo.last_login_at) }}
          </div>
          <div v-if="userStore.userInfo.last_login_ip" class="flex items-center gap-2">
            <div class="i-ep-location text-xs"></div> IP: {{ userStore.userInfo.last_login_ip }}
          </div>
        </div>
        <button
          type="button"
          class="inline-flex h-9 items-center gap-2 rounded-md border border-rose-500/20 bg-rose-500/10 px-3 text-xs font-bold text-rose-500 transition-colors hover:bg-rose-500/20"
          @click="handleLogout"
        >
          <div class="i-ep-switch-button text-sm"></div>
          <span>退出登录</span>
        </button>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-x-12 gap-y-12">
      <div class="space-y-12">

        <section class="space-y-5">
          <div class="flex items-center justify-between px-1">
            <div class="flex items-center gap-2">
              <div class="w-1.5 h-1.5 rounded-full bg-primary"></div>
              <h4 class="text-[13px] font-bold text-default">基本资料</h4>
            </div>
            <el-button type="primary" link @click="handleUpdateProfile" :loading="savingProfile" class="!h-auto !p-0 text-[11px] font-bold">
              保存修改
            </el-button>
          </div>
          <el-form :model="profileForm" :rules="profileRules" ref="profileFormRef" label-position="top">
            <el-form-item prop="username" class="!mb-5">
              <template #label>
                <span class="label-xs ml-1">用户名</span>
              </template>
              <el-input v-model="profileForm.username" placeholder="请输入新用户名" class="modern-input" />
            </el-form-item>
            <el-form-item required prop="password_confirm" class="!mb-0">
              <template #label>
                <span class="label-xs ml-1">验证当前密码</span>
              </template>
              <el-input v-model="profileForm.password_confirm" type="password" show-password placeholder="修改资料需验证身份" class="modern-input" />
            </el-form-item>
          </el-form>
        </section>

        <section class="space-y-5">
          <div class="flex items-center justify-between px-1">
            <div class="flex items-center gap-2">
              <div class="w-1.5 h-1.5 rounded-full bg-rose-500"></div>
              <h4 class="text-[13px] font-bold text-default">修改登录密码</h4>
            </div>
            <el-button type="danger" link @click="handleChangePassword" :loading="savingPass" class="!h-auto !p-0 text-[11px] font-bold">
              更新密码
            </el-button>
          </div>
          <el-form :model="passForm" :rules="passRules" ref="passFormRef" label-position="top" class="space-y-5">
            <el-form-item prop="old_password" class="!mb-0">
              <template #label>
                <span class="label-xs ml-1">当前密码</span>
              </template>
              <el-input v-model="passForm.old_password" type="password" show-password class="modern-input" />
            </el-form-item>
            <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <el-form-item prop="new_password" class="!mb-0">
                <template #label>
                  <span class="label-xs ml-1">新密码</span>
                </template>
                <el-input v-model="passForm.new_password" type="password" show-password class="modern-input" />
              </el-form-item>
              <el-form-item prop="confirm_password" class="!mb-0">
                <template #label>
                  <span class="label-xs ml-1">确认新密码</span>
                </template>
                <el-input v-model="passForm.confirm_password" type="password" show-password class="modern-input" />
              </el-form-item>
            </div>
          </el-form>
        </section>
      </div>

      <div class="space-y-12">

        <section v-if="userStore.userInfo.role === 'admin'" class="space-y-6">
          <div class="flex items-center gap-2 px-1">
            <div class="w-1.5 h-1.5 rounded-full bg-indigo-500"></div>
            <h4 class="text-[13px] font-bold text-default">全局安全策略</h4>
          </div>

          <div class="space-y-8">
            <div class="px-1">
              <div class="flex items-center justify-between mb-2">
                <span class="text-sm font-bold text-default">Telegram 登录二次验证 (2FA)</span>
                <el-switch v-model="tgConfig.login_2fa" @change="handleSaveTg2FA" :loading="savingTg2FA" />
              </div>
              <p class="text-[10px] text-muted leading-relaxed mb-3">
                开启后，每次从 Web 端登录时需在绑定的 Telegram 机器人上点击“允许”方可进入系统。
              </p>
              <div v-if="!tgConfig.enabled" class="inline-flex items-center gap-2 px-3 py-1.5 bg-amber-500/10 rounded-lg border border-amber-500/20">
                <div class="i-ep-warning text-amber-600 text-xs"></div>
                <span class="text-[10px] font-bold text-amber-700">Telegram 机器人未启用</span>
              </div>
            </div>

            <div class="px-1">
              <div class="flex items-center justify-between mb-2">
                <span class="text-sm font-bold text-default">全局最大并发会话数</span>
                <el-button type="primary" link @click="handleSaveMaxSessions" :loading="savingSecurity" class="!h-auto !p-0 text-[11px] font-bold">
                  应用限制
                </el-button>
              </div>
              <p class="text-[10px] text-muted leading-relaxed mb-4">同一个管理账户可同时登录的设备或终端数量上限。</p>
              <el-input-number v-model="maxSessions" :min="1" :max="10" class="!w-full max-w-[180px] modern-number" controls-position="right" size="small" />
            </div>
          </div>
        </section>

        <section class="space-y-5 flex-1 flex flex-col min-h-0">
          <div class="flex items-center justify-between px-1">
            <div class="flex items-center gap-2">
              <div class="w-1.5 h-1.5 rounded-full bg-emerald-500"></div>
              <h4 class="text-[13px] font-bold text-default">活跃会话（{{ sessions.length }}）</h4>
            </div>
            <el-button type="primary" link @click="loadSessions" :loading="loadingSessions" class="!h-auto !p-0 text-[11px] font-bold">
              刷新
            </el-button>
          </div>

          <div class="space-y-3 max-h-[300px] overflow-y-auto custom-scrollbar pr-2">
            <div v-if="sessions.length === 0" class="py-8 text-center text-xs text-muted opacity-50">暂无活跃会话</div>
            <div v-for="session in sessions" :key="session.id" class="rounded-md border border-light/50 bg-soft/5 p-4 transition-colors hover:bg-soft/10 dark:bg-white/[0.01]">
              <div class="flex justify-between items-center mb-3">
                <code class="text-[11px] font-bold text-primary opacity-80">{{ session.id.substring(0, 12) }}...</code>
                <el-tag v-if="session.is_current" size="small" effect="plain" class="!rounded-md border-none !bg-success/10 !text-success font-bold text-[10px]">当前会话</el-tag>
                <el-button v-else type="danger" link size="small" @click="handleRevoke(session.id)" class="!p-0 !h-auto text-[10px] font-bold">
                  撤销
                </el-button>
              </div>
              <div class="flex items-center gap-2 font-mono text-[10px] text-muted/60">
                <div class="i-ep-timer text-xs opacity-50"></div> 过期时间：{{ formatDate(session.expiry) }}
              </div>
            </div>
          </div>
        </section>
      </div>
    </div>
  </div>
</template>


<script setup lang="ts">
import { formatDate } from "../../../utils/format";
import { useSecuritySettings } from "./composables/useSecuritySettings";

const {
  handleChangePassword,
  handleLogout,
  handleRevoke,
  handleSaveMaxSessions,
  handleSaveTg2FA,
  handleUpdateProfile,
  loadSessions,
  loadingSessions,
  maxSessions,
  passForm,
  passFormRef,
  passRules,
  profileForm,
  profileFormRef,
  profileRules,
  savingPass,
  savingProfile,
  savingSecurity,
  savingTg2FA,
  sessions,
  tgConfig,
  userStore,
} = useSecuritySettings();
</script>
