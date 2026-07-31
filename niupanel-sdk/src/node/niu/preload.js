function autoGlobalsEnabled() {
    const value = String(process.env.NIUPANEL_SDK_AUTO_GLOBALS || "1").trim().toLowerCase();
    return !["0", "false", "no", "off"].includes(value);
}

if (autoGlobalsEnabled() && globalThis.niu === undefined) {
    Object.defineProperty(globalThis, "niu", {
        value: require("./index"),
        configurable: true,
        enumerable: false,
        writable: false,
    });
}
