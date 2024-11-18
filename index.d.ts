/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

export function getSteamPath(): string
export interface AppInfo {
  appid: number
  gamename: string
}
export function getAppInfo(): Array<AppInfo>
export function pressKey(key: number): void
export function getHqIcon(appid: number): string
export function depsInstalled(lib: string): string
export function hdrScreenshot(monitorId: number, sspath: string): string
export function getFocusedWinPath(): string
export namespace log {
  export function initLogger(appData: string): string
  export function testPanic(): void
}
