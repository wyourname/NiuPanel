import type { FileItem } from "./fileOperationTypes";

export const sortFileItems = (items: FileItem[]) => {
  return [...items].sort(
    (a, b) => (Number(b.is_dir) - Number(a.is_dir)) || a.name.localeCompare(b.name),
  );
};

export const joinDirectoryPath = (basePath: string, name: string) => {
  return `${basePath === "/" ? "" : `${basePath}/`}${name}`;
};

export const getParentPath = (path: string) => {
  return path.split("/").slice(0, -1).join("/");
};

export const getRenamedPath = (oldPath: string, newName: string) => {
  const parent = getParentPath(oldPath);
  return parent ? `${parent}/${newName}` : newName;
};

export const normalizeMoveTargetPath = (targetPath: string, itemName: string) => {
  const cleanTargetPath = targetPath.replace(/^\/+|\/+$/g, "");
  return cleanTargetPath ? `${cleanTargetPath}/${itemName}` : itemName;
};
