/**
 * Single source for the version string shown in the app chrome (status
 * footer, About surfaces, etc). Keep in sync with apps/desktop/package.json
 * and apps/desktop/src-tauri/tauri.conf.json when bumping — this constant
 * does not read those files at build time, it is hand-kept alongside them.
 */
export const APP_VERSION = "v0.1.0";
