import { chromium } from 'playwright';

const BASE_URL = process.env.E2E_BASE_URL || 'https://author.works';
const EMAIL = process.env.E2E_LOGTO_EMAIL;
const PASSWORD = process.env.E2E_LOGTO_PASSWORD;
const RUN_FULL_BOOK = (process.env.E2E_RUN_FULL_BOOK || 'false').toLowerCase() === 'true';
const POLL_TIMEOUT_MS = Number(process.env.E2E_POLL_TIMEOUT_MS || 600000);
const POLL_INTERVAL_MS = Number(process.env.E2E_POLL_INTERVAL_MS || 10000);

function requireEnv(name, value) {
  if (!value) {
    throw new Error(`Missing required environment variable: ${name}`);
  }
}

function withTimeout(signalMs = 30000) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), signalMs);
  return { controller, timeout };
}

async function api(path, { method = 'GET', token, body, timeoutMs = 30000 } = {}) {
  const { controller, timeout } = withTimeout(timeoutMs);
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

async function pollBookGeneration(jobId, token) {
  const started = Date.now();
  while (Date.now() - started < POLL_TIMEOUT_MS) {
    const statusCall = await api(`/api/generate/book/status/${jobId}`, {
      token,
      timeoutMs: 60000,
    });
    if (!statusCall.response.ok) {
      throw new Error(
        `Job status check failed (${statusCall.response.status}): ${statusCall.text}`
      );
    }

    const status = statusCall.json?.status;
    if (status === 'completed') {
      return statusCall.json;
    }
    if (status === 'failed' || status === 'cancelled') {
      throw new Error(`Book generation ended in status=${status}: ${statusCall.text}`);
    }

    await new Promise((r) => setTimeout(r, POLL_INTERVAL_MS));
  }

  throw new Error(`Timed out waiting for job completion after ${POLL_TIMEOUT_MS}ms`);
}

async function run() {
  requireEnv('E2E_LOGTO_EMAIL', EMAIL);
  requireEnv('E2E_LOGTO_PASSWORD', PASSWORD);

  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext();
  const page = await context.newPage();

  let createdBookId = null;

  try {
    await page.goto(BASE_URL, { waitUntil: 'domcontentloaded', timeout: 60000 });
    const signInButton = page.getByRole('button', { name: /sign in/i });
    await signInButton.click();

    await page.waitForURL(/auth\.leopaska\.xyz\/sign-in/, { timeout: 60000 });
    const inputs = page.getByRole('textbox');
    await inputs.nth(0).fill(EMAIL);
    await inputs.nth(1).fill(PASSWORD);
    await page.getByRole('button', { name: /sign in/i }).click();

    await page.waitForURL((url) => url.toString().startsWith(BASE_URL), {
      timeout: 120000,
    });
    await page.waitForTimeout(2000);

    const token = await page.evaluate(() => localStorage.getItem('accessToken'));
    requireEnv('accessToken after login', token);

    const bookTitle = `ci-e2e-${Date.now()}`;
    const createBook = await api('/api/books', {
      method: 'POST',
      token,
      body: {
        title: bookTitle,
        description: 'Automated CI E2E journey test',
        genre: 'fiction',
      },
    });
    if (!createBook.response.ok || !createBook.json?.id) {
      throw new Error(`Create book failed (${createBook.response.status}): ${createBook.text}`);
    }
    createdBookId = createBook.json.id;

    const createChapter = await api(`/api/books/${createdBookId}/chapters`, {
      method: 'POST',
      token,
      body: {
        title: 'CI E2E Chapter',
        content: 'Initial content from CI journey.',
      },
    });
    if (!createChapter.response.ok || !createChapter.json?.id) {
      throw new Error(
        `Create chapter failed (${createChapter.response.status}): ${createChapter.text}`
      );
    }

    const chapterId = createChapter.json.id;
    const updateChapter = await api(`/api/chapters/${chapterId}`, {
      method: 'PUT',
      token,
      body: {
        title: 'CI E2E Chapter Updated',
        content: 'Updated chapter content for CI journey verification.',
      },
    });
    if (!updateChapter.response.ok) {
      throw new Error(
        `Update chapter failed (${updateChapter.response.status}): ${updateChapter.text}`
      );
    }

    const outline = await api('/api/generate/outline', {
      method: 'POST',
      token,
      body: {
        book_id: createdBookId,
        prompt: 'Create a concise 3 chapter structure suitable for CI verification.',
        chapter_count: 3,
        genre: 'fiction',
      },
      timeoutMs: 180000,
    });
    if (!outline.response.ok) {
      throw new Error(`Generate outline failed (${outline.response.status}): ${outline.text}`);
    }

    if (RUN_FULL_BOOK) {
      const fullBook = await api('/api/generate/book', {
        method: 'POST',
        token,
        body: {
          book_id: createdBookId,
          title: bookTitle,
          description: 'CI full-book generation verification',
          genre: 'fiction',
          chapter_count: 3,
          outline_prompt: 'Short and coherent test story',
          author_name: 'CI Test Runner',
        },
        timeoutMs: 180000,
      });
      if (![200, 202].includes(fullBook.response.status) || !fullBook.json?.job_id) {
        throw new Error(
          `Start full-book generation failed (${fullBook.response.status}): ${fullBook.text}`
        );
      }
      await pollBookGeneration(fullBook.json.job_id, token);
    }

    const chaptersAfter = await api(`/api/books/${createdBookId}/chapters`, { token });
    if (!chaptersAfter.response.ok) {
      throw new Error(
        `List chapters failed (${chaptersAfter.response.status}): ${chaptersAfter.text}`
      );
    }
    const chapterCount = Array.isArray(chaptersAfter.json?.chapters)
      ? chaptersAfter.json.chapters.length
      : 0;
    if (chapterCount < 1) {
      throw new Error(`Expected at least one chapter after generation, got ${chapterCount}`);
    }

    console.log(
      JSON.stringify(
        {
          ok: true,
          baseUrl: BASE_URL,
          runFullBook: RUN_FULL_BOOK,
          createdBookId,
          finalChapterCount: chapterCount,
        },
        null,
        2
      )
    );
  } finally {
    if (createdBookId) {
      try {
        const token = await page.evaluate(() => localStorage.getItem('accessToken'));
        if (token) {
          await api(`/api/books/${createdBookId}`, { method: 'DELETE', token });
        }
      } catch {
        // Best-effort cleanup only.
      }
    }
    await context.close();
    await browser.close();
  }
}

run().catch((error) => {
  console.error(error);
  process.exit(1);
});
