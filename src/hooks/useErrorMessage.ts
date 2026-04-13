const normalizeErrorDetail = (error: unknown) => {
  if (typeof error === 'string') {
    return error;
  }

  if (error instanceof Error) {
    return error.message || error.name;
  }

  if (typeof error === 'object' && error !== null) {
    try {
      const serialized = JSON.stringify(error);

      if (serialized && serialized !== '{}') {
        return serialized;
      }
    } catch {
      return String(error);
    }
  }

  return String(error);
};

export const formatErrorMessage = (description: string, error: unknown) => {
  const detail = normalizeErrorDetail(error);

  return detail ? `${description}：${detail}` : description;
};
