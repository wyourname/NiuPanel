import { register } from "node:module";

register("./shared-dependencies-loader.mjs", import.meta.url);
