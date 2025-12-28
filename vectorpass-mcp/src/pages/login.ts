/**
 * Login page HTML - Email input
 */

export function renderLoginPage(sessionId: string, error?: string): string {
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>VectorPass - Connect to MCP</title>
  <script src="https://cdn.tailwindcss.com"></script>
  <style>
    .gradient-bg {
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    }
  </style>
</head>
<body class="min-h-screen gradient-bg flex items-center justify-center p-4">
  <div class="bg-white rounded-2xl shadow-2xl p-8 w-full max-w-md">
    <div class="text-center mb-8">
      <div class="w-16 h-16 bg-gradient-to-r from-indigo-500 to-purple-600 rounded-xl mx-auto mb-4 flex items-center justify-center">
        <svg class="w-10 h-10 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"></path>
        </svg>
      </div>
      <h1 class="text-2xl font-bold text-gray-800">Connect to VectorPass</h1>
      <p class="text-gray-500 mt-2">Sign in to authorize MCP access</p>
    </div>

    ${error ? `
    <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg mb-6">
      ${escapeHtml(error)}
    </div>
    ` : ''}

    <form action="/authorize" method="POST" class="space-y-6">
      <input type="hidden" name="session_id" value="${sessionId}">
      <input type="hidden" name="step" value="email">

      <div>
        <label for="email" class="block text-sm font-medium text-gray-700 mb-2">
          Email Address
        </label>
        <input
          type="email"
          id="email"
          name="email"
          required
          placeholder="you@example.com"
          class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition"
        >
      </div>

      <button
        type="submit"
        class="w-full bg-gradient-to-r from-indigo-500 to-purple-600 text-white py-3 px-4 rounded-lg font-semibold hover:from-indigo-600 hover:to-purple-700 transition transform hover:scale-[1.02]"
      >
        Continue
      </button>
    </form>

    <div class="mt-6 text-center text-sm text-gray-500">
      <p>We'll send you a verification code</p>
    </div>

    <div class="mt-8 pt-6 border-t border-gray-200 text-center">
      <p class="text-xs text-gray-400">
        By continuing, you authorize this application to access your VectorPass data.
      </p>
    </div>
  </div>
</body>
</html>`;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
