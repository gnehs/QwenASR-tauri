/// <reference types="vite/client" />

declare module "*.po" {
  import type { Messages } from "@lingui/core";

  export const messages: Messages;
}

declare const __APP_VERSION__: string;
declare const __COMMIT_SHA__: string;
