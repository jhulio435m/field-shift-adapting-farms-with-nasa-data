import type { User } from "./api";
export type View =
  | "home"
  | "chat"
  | "assets"
  | "jobs"
  | "publications"
  | "users"
  | "sessions"
  | "connections"
  | "pilot"
  | "academic"
  | "resourceAccess"
  | "integrations"
  | "activity"
  | "workspaces"
  | "cases";
export type Context = {
  user: User;
  run: (action: () => Promise<void>) => Promise<void>;
  revision: number;
  updated: () => void;
  go: (v: View, id?: string) => void;
  selected: string;
  setNotice: (s: string) => void;
};
