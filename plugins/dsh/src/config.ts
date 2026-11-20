import Schema from '@deepseek-ai/schemastery'

export interface Config {
  executable: string
  executableSha256?: string
  cliVersion: string
  workspaceRoot: string
  worker?: string
  voiceRegistry?: string
  voicePackage?: string
  projectRoot?: string
  onnxruntimeLibrary?: string
  sessionPolicy: 'private' | 'shared'
  maxActiveJobs: number
  maxWorkspaces: number
  maxAttachments: number
  maxRunsPerWorkspace: number
  maxArtifactsPerWorkspace: number
  maxAttachmentBytes: number
  maxArtifactBytes: number
  maxWorkspaceBytes: number
  maxRequestBytes: number
  maxResponseBytes: number
  maxStderrBytes: number
  maxOperations: number
  queryTimeoutMs: number
  renderTimeoutMs: number
  killGraceMs: number
  maxRenderSeconds: number
  retentionHours: number
  desktopUrl?: string
}

export const Config: Schema<Config> = Schema.object({
  executable: Schema.string().default('verdant-cli'),
  executableSha256: Schema.string(),
  cliVersion: Schema.string().default('0.1.0'),
  workspaceRoot: Schema.string().default('./verdant-dsh-workspaces'),
  worker: Schema.string(),
  voiceRegistry: Schema.string(),
  voicePackage: Schema.string(),
  projectRoot: Schema.string(),
  onnxruntimeLibrary: Schema.string(),
  sessionPolicy: Schema.union(['private', 'shared'] as const).default('private'),
  maxActiveJobs: Schema.natural().min(1).max(16).default(2),
  maxWorkspaces: Schema.natural().min(1).max(10000).default(128),
  maxAttachments: Schema.natural().min(1).max(128).default(16),
  maxRunsPerWorkspace: Schema.natural().min(1).max(1024).default(64),
  maxArtifactsPerWorkspace: Schema.natural().min(1).max(4096).default(256),
  maxAttachmentBytes: Schema.natural().min(1).default(16 * 1024 * 1024),
  maxArtifactBytes: Schema.natural().min(1).default(256 * 1024 * 1024),
  maxWorkspaceBytes: Schema.natural().min(1).default(512 * 1024 * 1024),
  maxRequestBytes: Schema.natural().min(1024).max(2 * 1024 * 1024).default(1024 * 1024),
  maxResponseBytes: Schema.natural().min(1024).default(4 * 1024 * 1024),
  maxStderrBytes: Schema.natural().min(1024).default(64 * 1024),
  maxOperations: Schema.natural().min(1).max(10000).default(1024),
  queryTimeoutMs: Schema.natural().min(100).default(30000),
  renderTimeoutMs: Schema.natural().min(100).default(600000),
  killGraceMs: Schema.natural().min(100).max(30000).default(3000),
  maxRenderSeconds: Schema.natural().min(1).default(600),
  retentionHours: Schema.natural().min(1).default(168),
  desktopUrl: Schema.string(),
})
