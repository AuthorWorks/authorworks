import { NextRequest, NextResponse } from 'next/server'
import { getUserId, unauthorized } from '@/app/lib/auth'
import {
  deleteChapterForUser,
  findChapterForUser,
  logChapterEvent,
  updateChapterForUser,
} from '@/app/lib/chapters'
import { getPool } from '@/app/lib/db'

interface RouteContext {
  params: { id: string }
}

// GET /api/chapters/[id] - Fetch a chapter, scoped to the authenticated owner.
export async function GET(request: NextRequest, { params }: RouteContext) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  const pool = getPool()
  try {
    const chapter = await findChapterForUser(pool, params.id, userId, { withBookTitle: true })
    if (!chapter) {
      return NextResponse.json({ error: 'Chapter not found' }, { status: 404 })
    }
    return NextResponse.json(chapter)
  } catch (error) {
    console.error('GET /api/chapters/[id] failed:', error)
    return NextResponse.json({ error: 'Failed to fetch chapter' }, { status: 500 })
  }
}

// PUT /api/chapters/[id] - Update a chapter and recompute book word counts.
export async function PUT(request: NextRequest, { params }: RouteContext) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  let body: { title?: string | null; content?: string | null }
  try {
    body = await request.json()
  } catch {
    return NextResponse.json({ error: 'Invalid JSON body' }, { status: 400 })
  }

  const pool = getPool()
  try {
    const updated = await updateChapterForUser(pool, params.id, userId, body)
    if (!updated) {
      return NextResponse.json({ error: 'Chapter not found' }, { status: 404 })
    }

    await logChapterEvent(
      pool,
      updated.book_id as string,
      'edit',
      `Chapter "${updated.title ?? `#${updated.chapter_number}`}" edited`,
      {
        chapter_id: params.id,
        word_count: updated.word_count,
        user_id: userId,
        timestamp: new Date().toISOString(),
      }
    )

    return NextResponse.json(updated)
  } catch (error) {
    console.error('PUT /api/chapters/[id] failed:', error)
    return NextResponse.json({ error: 'Failed to update chapter' }, { status: 500 })
  }
}

// DELETE /api/chapters/[id] - Delete a chapter and renumber siblings.
export async function DELETE(request: NextRequest, { params }: RouteContext) {
  const userId = await getUserId(request)
  if (!userId) return unauthorized()

  const pool = getPool()
  try {
    const result = await deleteChapterForUser(pool, params.id, userId)
    if (!result) {
      return NextResponse.json({ error: 'Chapter not found' }, { status: 404 })
    }

    await logChapterEvent(
      pool,
      result.bookId,
      'delete',
      `Chapter "${result.chapterTitle ?? params.id}" deleted`,
      {
        chapter_id: params.id,
        user_id: userId,
        timestamp: new Date().toISOString(),
      }
    )

    return NextResponse.json({ success: true })
  } catch (error) {
    console.error('DELETE /api/chapters/[id] failed:', error)
    return NextResponse.json({ error: 'Failed to delete chapter' }, { status: 500 })
  }
}
