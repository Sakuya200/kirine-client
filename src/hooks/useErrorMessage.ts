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

const MAX_ERROR_MESSAGE_CHARS = 280;

const truncateErrorMessage = (value: string) => {
  const trimmed = value.trim();

  if (trimmed.length <= MAX_ERROR_MESSAGE_CHARS) {
    return trimmed;
  }

  return `${trimmed.slice(0, MAX_ERROR_MESSAGE_CHARS)}...`;
};

export const formatErrorMessage = (description: string, error: unknown) => {
  const detail = truncateErrorMessage(normalizeErrorDetail(error));

  return detail ? `${description}：${detail}` : description;
};
