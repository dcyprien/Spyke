export async function authFetch(
  url: string,
  options: RequestInit = {}
) {
  const token = localStorage.getItem("access_token");

  return fetch(url, {
    ...options,
    headers: {
      ...(options.headers || {}),
      "Content-Type": "application/json",
      Authorization: token ? `Bearer ${token}` : "",
    },
  });
}
