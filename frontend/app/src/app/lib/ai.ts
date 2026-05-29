/**
 * Provider-agnostic chat completion helper.
 *
 * Defaults to the homelab self-hosted LLM gateway (LiteLLM, OpenAI-compatible)
 * so no traffic leaves the cluster unless an operator opts in. Hosted API-key
 * providers remain available as an explicit override.
 *
 * Selected via environment variables:
 *  - AI_PROVIDER=litellm (default) — OpenAI-compatible gateway
 *      AI_BASE_URL  (default http://litellm.inference.svc.cluster.local:4000/v1)
 *      AI_MODEL     (default "chat")
 *      AI_API_KEY   (Bearer token; falls back to LITELLM_API_KEY / OPENAI_API_KEY)
 *  - AI_PROVIDER=ollama — OpenAI-compatible local Ollama (OLLAMA_BASE_URL)
 *  - AI_PROVIDER=anthropic + ANTHROPIC_API_KEY — Anthropic Messages API
 */

export type AiProvider = 'anthropic' | 'openai'

export interface ChatMessage {
  role: 'system' | 'user' | 'assistant'
  content: string
}

export interface ChatCompletionResult {
  text: string
  inputTokens: number
  outputTokens: number
  model: string
  provider: AiProvider
}

interface ProviderConfig {
  provider: AiProvider
  model: string
  baseUrl: string
  apiKey?: string
}

const DEFAULT_LITELLM_BASE_URL = 'http://litellm.inference.svc.cluster.local:4000/v1'
const DEFAULT_OLLAMA_BASE_URL = 'http://192.168.1.200:11434'

function stripTrailingSlash(url: string): string {
  return url.replace(/\/+$/, '')
}

export function getAiConfig(): ProviderConfig {
  const explicit = (process.env.AI_PROVIDER || 'litellm').toLowerCase()
  const anthropicApiKey = process.env.ANTHROPIC_API_KEY
  const useAnthropic = explicit === 'anthropic' && Boolean(anthropicApiKey)

  if (useAnthropic) {
    return {
      provider: 'anthropic',
      model: process.env.ANTHROPIC_MODEL || 'claude-sonnet-4-20250514',
      baseUrl: 'https://api.anthropic.com',
      apiKey: anthropicApiKey,
    }
  }

  // Every other provider speaks the OpenAI chat-completions protocol.
  // Ollama keeps its dedicated base URL for backward compatibility; all other
  // values resolve to the LiteLLM gateway unless AI_BASE_URL overrides it.
  const isOllama = explicit === 'ollama'
  const baseUrl = stripTrailingSlash(
    process.env.AI_BASE_URL ||
      (isOllama ? `${stripTrailingSlash(process.env.OLLAMA_BASE_URL || DEFAULT_OLLAMA_BASE_URL)}/v1` : DEFAULT_LITELLM_BASE_URL)
  )

  return {
    provider: 'openai',
    model: process.env.AI_MODEL || (isOllama ? 'deepseek-coder-v2:16b' : 'chat'),
    baseUrl,
    apiKey:
      process.env.AI_API_KEY ||
      process.env.LITELLM_API_KEY ||
      process.env.OPENAI_API_KEY ||
      undefined,
  }
}

export async function chatCompletion(
  systemPrompt: string,
  userPrompt: string,
  options: { maxTokens?: number; temperature?: number } = {}
): Promise<ChatCompletionResult> {
  const config = getAiConfig()
  const maxTokens = options.maxTokens ?? 8000
  const temperature = options.temperature ?? 0.7

  if (config.provider === 'anthropic') {
    const response = await fetch(`${config.baseUrl}/v1/messages`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-api-key': config.apiKey!,
        'anthropic-version': '2023-06-01',
      },
      body: JSON.stringify({
        model: config.model,
        max_tokens: maxTokens,
        system: systemPrompt,
        messages: [{ role: 'user', content: userPrompt }],
      }),
    })

    if (!response.ok) {
      throw new AiError(await response.text(), response.status)
    }

    const data = await response.json()
    return {
      text: data.content?.[0]?.text ?? '',
      inputTokens: data.usage?.input_tokens ?? 0,
      outputTokens: data.usage?.output_tokens ?? 0,
      model: config.model,
      provider: 'anthropic',
    }
  }

  const headers: Record<string, string> = { 'Content-Type': 'application/json' }
  if (config.apiKey) {
    headers.Authorization = `Bearer ${config.apiKey}`
  }

  const response = await fetch(`${config.baseUrl}/chat/completions`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      model: config.model,
      messages: [
        { role: 'system', content: systemPrompt },
        { role: 'user', content: userPrompt },
      ],
      max_tokens: maxTokens,
      temperature,
      stream: false,
    }),
  })

  if (!response.ok) {
    throw new AiError(await response.text(), response.status)
  }

  const data = await response.json()
  return {
    text: data.choices?.[0]?.message?.content ?? '',
    inputTokens: data.usage?.prompt_tokens ?? 0,
    outputTokens: data.usage?.completion_tokens ?? 0,
    model: config.model,
    provider: 'openai',
  }
}

export class AiError extends Error {
  constructor(message: string, public status: number) {
    super(message)
    this.name = 'AiError'
  }
}

/** Extracts a JSON object from a string that may include ```json``` code fences. */
export function extractJson<T>(raw: string): T {
  const fenced = raw.match(/```(?:json)?\s*([\s\S]*?)```/)
  const candidate = fenced ? fenced[1] : raw
  return JSON.parse(candidate.trim()) as T
}
