export type RunnerKind = "proton" | "wine";

export interface Runner {
  id: string;
  name: string;
  path: string;
  kind: RunnerKind;
}

export interface PrefixInfo {
  path: string;
}

export interface Game {
  id: string;
  name: string;
  exe_path: string;
  prefix_path: string;
  runner_id: string;
  env_vars: Record<string, string>;
}

export interface GameInput {
  name: string;
  exe_path: string;
  prefix_path: string;
  runner_id: string;
  env_vars: Record<string, string>;
}

export interface ProtonGeRelease {
  tag: string;
  name: string;
  published_at: string;
  download_url: string;
  size: number;
}
