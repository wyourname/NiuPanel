export type EnvType = "python" | "node" | "sh";
export type InstallableEnvType = Extract<EnvType, "python" | "node">;

export interface Env {
  name: string;
  env_type: EnvType;
  version?: string;
  path?: string;
  is_installed?: boolean;
  recorded_packages?: number;
}

export interface Package {
  name: string;
  version: string;
}

export type PackageDependencyInfo = {
  version?: string;
};

export type PackageDependencyMap = Record<
  string,
  PackageDependencyInfo | string
>;

export type PackageListPayload =
  | Package[]
  | string
  | {
      dependencies?: PackageDependencyMap;
    };

export type InstallPackagesRequest = {
  packages: string[];
};
