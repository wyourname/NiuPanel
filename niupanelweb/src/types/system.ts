export type PluginSandboxMode = 'full' | 'compatible' | 'degraded' | 'unsupported'

export interface PluginSandboxCapability {
  mode: PluginSandboxMode
  landlock_abi?: number | null
  uid_isolation: boolean
  seccomp: boolean
  no_new_privs: boolean
}

export interface SystemVersionInfo {
  panel_version: string
  api_contract: number
  schema_epoch: number
  schema_revision: number
  plugin_sandbox: PluginSandboxCapability
}

export interface PanelActivationFailure {
  transaction_id: string
  version: string
  failed_at: string
  message: string
}

export interface PanelReleaseRecord {
  version: string
  active: boolean
  previous: boolean
  rollback_available: boolean
  installed_at: string
}

export interface PanelReleaseList {
  launcher_managed: boolean
  active_version: string
  previous_version?: string | null
  pending_version?: string | null
  last_failure?: PanelActivationFailure | null
  releases: PanelReleaseRecord[]
}

export interface PanelReleaseMutation {
  version: string
  transaction_id: string
  database_restore_required: boolean
  message: string
}
