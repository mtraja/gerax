export const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:8080';

export async function request(path, options = {}) {
  const url = `${API_BASE}${path}`;
  const config = {
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
    ...options,
  };

  const response = await fetch(url, config);

  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || `Erro na requisição: ${response.status}`);
  }

  if (response.status === 204) {
    return null;
  }

  return response.json();
}
