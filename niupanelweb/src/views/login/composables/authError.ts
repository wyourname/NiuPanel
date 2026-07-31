export const getApiErrorStatus = (error: unknown) => {
  if (typeof error !== "object" || error === null || !("response" in error)) return undefined;
  return (error as { response?: { status?: number } }).response?.status;
};

export const getApiErrorMessage = (error: unknown, fallback: string) => {
  if (typeof error !== "object" || error === null || !("response" in error)) return fallback;
  return (error as { response?: { data?: { message?: string } } }).response?.data?.message || fallback;
};
