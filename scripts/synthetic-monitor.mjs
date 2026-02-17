const BASE_URL = process.env.SYNTHETIC_BASE_URL || 'https://author.works';
const TOKEN = process.env.SYNTHETIC_BEARER_TOKEN;

function requireEnv(name, value) {
  if (!value) {
    throw new Error(`Missing required environment variable: ${name}`);
  }
}

async function request(path, { method = 'GET', token, body, timeoutMs = 30000 } = {}) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetch(`${BASE_URL}${path}`, {
      method,
      signal: controller.signal,
      headers: {
        'Content-Type': 'application/json',
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
      },
      ...(body ? { body: JSON.stringify(body) } : {}),
    });
    const text = await response.text();
    let json = null;
    try {
      json = text ? JSON.parse(text) : null;
    } catch {
      json = null;
    }
    return { response, text, json };
  } finally {
    clearTimeout(timeout);
  }
}

function assertStatus(name, status, accepted) {
  if (!accepted.includes(status)) {
    throw new Error(`${name} expected ${accepted.join('/')} but got ${status}`);
  }
}

async function run() {
  requireEnv('SYNTHETIC_BEARER_TOKEN', TOKEN);

  const result = {
    ok: true,
    baseUrl: BASE_URL,
    checks: [],
  };

  let createdBookId = null;
  let createdChapterId = null;

  try {
    const home = await request('/');
    assertStatus('home', home.response.status, [200]);
    result.checks.push({ name: 'home', status: home.response.status });

    const metrics = await request('/api/metrics');
    assertStatus('frontend metrics', metrics.response.status, [200]);
    result.checks.push({ name: 'frontend_metrics', status: metrics.response.status });

    const authMe = await request('/api/auth/me', { token: TOKEN });
    assertStatus('auth me', authMe.response.status, [200]);
    result.checks.push({ name: 'auth_me', status: authMe.response.status });

    const dashboard = await request('/api/dashboard/stats', { token: TOKEN });
    assertStatus('dashboard stats', dashboard.response.status, [200]);
    result.checks.push({ name: 'dashboard_stats', status: dashboard.response.status });

    const createBook = await request('/api/books', {
      method: 'POST',
      token: TOKEN,
      body: {
        title: `synthetic-${Date.now()}`,
        description: 'Synthetic monitor book',
        genre: 'fiction',
      },
    });
    assertStatus('create book', createBook.response.status, [200, 201]);
    if (!createBook.json?.id) {
      throw new Error('create book response missing id');
    }
    createdBookId = createBook.json.id;
    result.checks.push({ name: 'create_book', status: createBook.response.status });

    const createChapter = await request(`/api/books/${createdBookId}/chapters`, {
      method: 'POST',
      token: TOKEN,
      body: {
        title: 'Synthetic chapter',
        content: 'Synthetic monitor content.',
      },
    });
    assertStatus('create chapter', createChapter.response.status, [200, 201]);
    if (!createChapter.json?.id) {
      throw new Error('create chapter response missing id');
    }
    createdChapterId = createChapter.json.id;
    result.checks.push({ name: 'create_chapter', status: createChapter.response.status });

    const updateChapter = await request(`/api/chapters/${createdChapterId}`, {
      method: 'PUT',
      token: TOKEN,
      body: {
        title: 'Synthetic chapter updated',
        content: 'Updated synthetic monitor content.',
      },
    });
    assertStatus('update chapter', updateChapter.response.status, [200]);
    result.checks.push({ name: 'update_chapter', status: updateChapter.response.status });

    const listChapters = await request(`/api/books/${createdBookId}/chapters`, { token: TOKEN });
    assertStatus('list chapters', listChapters.response.status, [200]);
    const chapterCount = Array.isArray(listChapters.json?.chapters)
      ? listChapters.json.chapters.length
      : 0;
    if (chapterCount < 1) {
      throw new Error('list chapters returned zero rows for synthetic book');
    }
    result.checks.push({
      name: 'list_chapters',
      status: listChapters.response.status,
      chapterCount,
    });

    console.log(JSON.stringify(result, null, 2));
  } finally {
    if (createdBookId) {
      try {
        await request(`/api/books/${createdBookId}`, {
          method: 'DELETE',
          token: TOKEN,
        });
      } catch {
        // Best-effort cleanup only.
      }
    }
  }
}

run().catch((error) => {
  console.error(error);
  process.exit(1);
});
