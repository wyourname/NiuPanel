export interface ComponentCompatibility {
  min: string
  max?: string | null
}

export interface WebReleaseManifest {
  component: 'web'
  version: string
  api_contract: number
  core: ComponentCompatibility
  built_at?: string | null
  files: Record<string, string>
}

export interface SystemVersionInfo {
  core_version: string
  web_version: string
  api_contract: number
  schema_epoch: number
  schema_revision: number
  web_compatible: boolean
  web_compatibility_error?: string | null
  web_manifest?: WebReleaseManifest | null
}

export interface WebReleaseRecord {
  version: string
  active: boolean
  previous: boolean
  managed: boolean
  compatible: boolean
  compatibility_error?: string | null
  manifest?: WebReleaseManifest | null
}

export interface WebReleaseList {
  active_version: string
  previous_version?: string | null
  releases: WebReleaseRecord[]
}

export interface WebReleaseMutation {
  version: string
  active: boolean
  message: string
}

export interface WebUpdateInfo {
  version: string
  current_version: string
  release_tag: string
  html_url: string
  body: string
  channel: string
  prerelease: boolean
  update_available: boolean
  size: number
}

export interface CoreActivationFailure {
  transaction_id: string
  version: string
  failed_at: string
  message: string
}

export interface CoreReleaseRecord {
  version: string
  active: boolean
  previous: boolean
  launcher_protocol: number
  api_contract: number
  schema_epoch: number
  schema_revision: number
  target: string
  installed_at: string
}

export interface CoreReleaseList {
  launcher_managed: boolean
  active_version: string
  previous_version?: string | null
  pending_version?: string | null
  last_failure?: CoreActivationFailure | null
  releases: CoreReleaseRecord[]
}

export interface CoreReleaseMutation {
  version: string
  transaction_id: string
  database_restore_required: boolean
  message: string
}
