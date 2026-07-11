import type { Component } from "vue";
import type { WorkspaceAppId } from "@/types/workspace";
import Overview from "@/views/modules/overview/index.vue";
import Tasks from "@/views/modules/Tasks.vue";
import Variables from "@/views/modules/Variables.vue";
import File from "@/views/modules/File.vue";
import Environment from "@/views/modules/Environment.vue";
import Share from "@/views/modules/share/index.vue";
import Extensions from "@/views/modules/extensions/index.vue";
import Git from "@/views/modules/git/index.vue";
import Telegram from "@/views/modules/telegram/index.vue";
import Terminal from "@/views/modules/terminal/index.vue";
import Settings from "@/views/modules/settings/index.vue";
import More from "@/views/modules/more/index.vue";

export const workspaceAppComponents: Partial<Record<WorkspaceAppId, Component>> = {
  overview: Overview,
  tasks: Tasks,
  variables: Variables,
  files: File,
  environments: Environment,
  share: Share,
  extensions: Extensions,
  git: Git,
  telegram: Telegram,
  terminal: Terminal,
  settings: Settings,
  more: More,
};
