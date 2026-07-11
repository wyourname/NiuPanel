export type VariableTextItem = {
  key: string;
  value: string;
};

export const parseVariableText = (text: string): VariableTextItem[] => {
  const lines = text.split("\n");
  const list: VariableTextItem[] = [];
  let currentKey = "";
  let currentValue = "";

  for (const line of lines) {
    if (line.includes("=")) {
      if (currentKey) {
        list.push({ key: currentKey.trim(), value: currentValue.trim() });
      }

      const [key, ...valueParts] = line.split("=");
      currentKey = key;
      currentValue = valueParts.join("=");
    } else if (currentKey) {
      currentValue += "\n" + line;
    }
  }

  if (currentKey) {
    list.push({ key: currentKey.trim(), value: currentValue.trim() });
  }

  return list;
};

export const formatVariableText = (list: VariableTextItem[]) => {
  return list
    .filter((item) => item.key)
    .map((item) => `${item.key}=${item.value}`)
    .join("\n");
};
