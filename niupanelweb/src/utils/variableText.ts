export type VariableTextItem = {
  key: string;
  value: string;
};

export const parseVariableText = (text: string): VariableTextItem[] => {
  const lines = text.split("\n");
  const list: VariableTextItem[] = [];

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;

    const match = line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=(.*)$/);
    if (!match) {
      throw new Error(`第 ${index + 1} 行不是有效的 KEY=VALUE`);
    }

    const [, key, rawValue] = match;
    const valueText = rawValue.trim();
    let value = valueText;

    if (valueText.startsWith('"')) {
      try {
        value = JSON.parse(valueText);
      } catch {
        throw new Error(
          `第 ${index + 1} 行的双引号值无效；多行请使用 \\n 转义`,
        );
      }
      if (typeof value !== "string") {
        throw new Error(`第 ${index + 1} 行的值必须是字符串`);
      }
    } else if (valueText.startsWith("'")) {
      if (!valueText.endsWith("'") || valueText.length < 2) {
        throw new Error(`第 ${index + 1} 行的单引号没有闭合`);
      }
      value = valueText.slice(1, -1);
    }

    list.push({ key, value });
  }

  return list;
};

export const formatVariableText = (list: VariableTextItem[]) => {
  return list
    .filter((item) => item.key)
    .map((item) => `${item.key}=${JSON.stringify(item.value)}`)
    .join("\n");
};
