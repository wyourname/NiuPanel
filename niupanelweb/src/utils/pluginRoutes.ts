export type NormalizedPluginRoute = {
  routePath: string;
  routeQuery: Record<string, unknown>;
};

export const queryFromSearch = (search: string): Record<string, unknown> => {
  const params = new URLSearchParams(search.replace(/^\?/, ""));
  const query: Record<string, unknown> = {};

  params.forEach((value, key) => {
    const current = query[key];
    if (Array.isArray(current)) {
      current.push(value);
    } else if (typeof current === "string") {
      query[key] = [current, value];
    } else {
      query[key] = value;
    }
  });

  return query;
};

export const normalizePluginRoute = (
  pluginId: string,
  path?: string | null,
): NormalizedPluginRoute => {
  const raw = path || `/plugins/${pluginId}`;
  const [pathname, search = ""] = raw.split("?");
  const pluginPrefix = `/plugins/${pluginId}`;
  let routePath = pathname;

  if (routePath === pluginPrefix) {
    routePath = "";
  } else if (routePath.startsWith(`${pluginPrefix}/`)) {
    routePath = routePath.slice(pluginPrefix.length + 1);
  } else {
    routePath = routePath.replace(/^\/+/, "");
  }

  return {
    routePath,
    routeQuery: queryFromSearch(search),
  };
};

export const pluginRoutePath = (pluginId: string, path: string) =>
  normalizePluginRoute(pluginId, path).routePath;
