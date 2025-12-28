/**
 * Verification page HTML - Code input
 */

export function renderVerifyPage(sessionId: string, email: string, error?: string): string {
  const maskedEmail = maskEmail(email);

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>VectorPass - Verify Code</title>
  <script src="https://cdn.tailwindcss.com"></script>
  <style>
    .gradient-bg {
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    }
    .code-input {
      letter-spacing: 0.5em;
      text-align: center;
      font-family: monospace;
    }
  </style>
</head>
<body class="min-h-screen gradient-bg flex items-center justify-center p-4">
  <div class="bg-white rounded-2xl shadow-2xl p-8 w-full max-w-md">
    <div class="text-center mb-8">
      <div class="w-16 h-16 bg-gradient-to-r from-green-400 to-emerald-500 rounded-xl mx-auto mb-4 flex items-center justify-center">
        <svg class="w-10 h-10 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"></path>
        </svg>
      </div>
      <h1 class="text-2xl font-bold text-gray-800">Check your email</h1>
      <p class="text-gray-500 mt-2">
        We sent a verification code to<br>
        <span class="font-medium text-gray-700">${escapeHtml(maskedEmail)}</span>
      </p>
    </div>

    ${error ? `
    <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg mb-6">
      ${escapeHtml(error)}
    </div>
    ` : ''}

    <form action="/authorize" method="POST" class="space-y-6">
      <input type="hidden" name="session_id" value="${sessionId}">
      <input type="hidden" name="step" value="verify">

      <div>
        <label for="code" class="block text-sm font-medium text-gray-700 mb-2">
          Verification Code
        </label>
        <input
          type="text"
          id="code"
          name="code"
          required
          maxlength="6"
          pattern="[0-9]{6}"
          placeholder="000000"
          autocomplete="one-time-code"
          class="code-input w-full px-4 py-4 text-2xl border border-gray-300 rounded-lg focus:ring-2 focus:ring-green-500 focus:border-green-500 transition"
        >
      </div>

      <button
        type="submit"
        class="w-full bg-gradient-to-r from-green-400 to-emerald-500 text-white py-3 px-4 rounded-lg font-semibold hover:from-green-500 hover:to-emerald-600 transition transform hover:scale-[1.02]"
      >
        Verify & Connect
      </button>
    </form>

    <div class="mt-6 text-center">
      <form action="/authorize" method="POST" class="inline">
        <input type="hidden" name="session_id" value="${sessionId}">
        <input type="hidden" name="step" value="resend">
        <button type="submit" class="text-sm text-indigo-600 hover:text-indigo-800 font-medium">
          Resend code
        </button>
      </form>
    </div>

    <div class="mt-4 text-center">
      <a href="/authorize?restart=${sessionId}" class="text-sm text-gray-500 hover:text-gray-700">
        Use a different email
      </a>
    </div>

    <div class="mt-8 pt-6 border-t border-gray-200 text-center">
      <p class="text-xs text-gray-400">
        Code expires in 15 minutes
      </p>
    </div>
  </div>

  <script>
    // Auto-focus and format code input
    const codeInput = document.getElementById('code');
    codeInput.focus();
    codeInput.addEventListener('input', (e) => {
      e.target.value = e.target.value.replace(/[^0-9]/g, '').slice(0, 6);
    });
  </script>
</body>
</html>`;
}

function maskEmail(email: string): string {
  const [local, domain] = email.split('@');
  if (local.length <= 2) {
    return `${local[0]}***@${domain}`;
  }
  return `${local[0]}${local[1]}***@${domain}`;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
