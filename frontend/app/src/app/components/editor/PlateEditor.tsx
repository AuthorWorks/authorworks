'use client'

import {
  Bold,
  Heading1,
  Heading2,
  Heading3,
  Italic,
  List,
  ListOrdered,
  Loader2,
  Quote,
  Sparkles,
  Underline,
  type LucideIcon,
} from 'lucide-react'
import { useCallback, useMemo, useState } from 'react'
import {
  BaseEditor,
  createEditor,
  Descendant,
  Editor,
  Element as SlateElement,
  Text,
  Transforms,
} from 'slate'
import { HistoryEditor, withHistory } from 'slate-history'
import {
  Editable,
  ReactEditor,
  RenderElementProps,
  RenderLeafProps,
  Slate,
  useSlate,
  withReact,
} from 'slate-react'
import { cn } from '../../lib/utils'

// ---------------------------------------------------------------------------
// Slate type definitions
// ---------------------------------------------------------------------------

type CustomText = {
  text: string
  bold?: boolean
  italic?: boolean
  underline?: boolean
}

type ParagraphElement = { type: 'paragraph'; children: CustomText[] }
type HeadingElement = {
  type: 'heading-one' | 'heading-two' | 'heading-three'
  children: CustomText[]
}
type BlockquoteElement = { type: 'block-quote'; children: CustomText[] }
type ListElement = {
  type: 'bulleted-list' | 'numbered-list'
  children: ListItemElement[]
}
type ListItemElement = { type: 'list-item'; children: CustomText[] }

type CustomElement =
  | ParagraphElement
  | HeadingElement
  | BlockquoteElement
  | ListElement
  | ListItemElement
type CustomEditor = BaseEditor & ReactEditor & HistoryEditor

declare module 'slate' {
  interface CustomTypes {
    Editor: CustomEditor
    Element: CustomElement
    Text: CustomText
  }
}

type MarkFormat = keyof Omit<CustomText, 'text'>
type BlockFormat = CustomElement['type']

// ---------------------------------------------------------------------------
// Inline mark <-> markdown helpers
// ---------------------------------------------------------------------------

function serializeInline(text: CustomText): string {
  let value = text.text
  if (!value) return ''
  if (text.bold) value = `**${value}**`
  if (text.italic) value = `*${value}*`
  if (text.underline) value = `<u>${value}</u>`
  return value
}

const INLINE_PATTERNS: Array<{ regex: RegExp; mark: MarkFormat }> = [
  { regex: /\*\*([^*]+)\*\*/g, mark: 'bold' },
  { regex: /(?<!\*)\*([^*\n]+)\*(?!\*)/g, mark: 'italic' },
  { regex: /<u>([^<]+)<\/u>/g, mark: 'underline' },
]

/** Splits a markdown line into a list of Slate text nodes preserving inline marks. */
function deserializeInline(line: string): CustomText[] {
  type Span = { text: string; marks: Set<MarkFormat> }
  let spans: Span[] = [{ text: line, marks: new Set<MarkFormat>() }]

  for (const { regex, mark } of INLINE_PATTERNS) {
    const next: Span[] = []
    for (const span of spans) {
      if (span.marks.has(mark)) {
        next.push(span)
        continue
      }
      let cursor = 0
      let match: RegExpExecArray | null
      const matchedRegex = new RegExp(regex.source, regex.flags)
      while ((match = matchedRegex.exec(span.text)) !== null) {
        if (match.index > cursor) {
          next.push({ text: span.text.slice(cursor, match.index), marks: new Set(span.marks) })
        }
        const inner = match[1]
        const innerMarks = new Set(span.marks)
        innerMarks.add(mark)
        next.push({ text: inner, marks: innerMarks })
        cursor = match.index + match[0].length
      }
      if (cursor < span.text.length) {
        next.push({ text: span.text.slice(cursor), marks: new Set(span.marks) })
      }
    }
    spans = next
  }

  const result = spans
    .filter((span) => span.text.length > 0)
    .map<CustomText>((span) => {
      const node: CustomText = { text: span.text }
      span.marks.forEach((m) => {
        node[m] = true
      })
      return node
    })
  return result.length > 0 ? result : [{ text: '' }]
}

// ---------------------------------------------------------------------------
// Markdown <-> Slate serialization
// ---------------------------------------------------------------------------

export function serializeToMarkdown(nodes: Descendant[]): string {
  return nodes.map((node) => serializeNode(node)).join('\n\n')
}

function serializeNode(node: Descendant): string {
  if (Text.isText(node)) return serializeInline(node)

  if (node.type === 'bulleted-list' || node.type === 'numbered-list') {
    return node.children
      .map((child, index) => {
        const inner = (child.children as CustomText[]).map(serializeInline).join('')
        return node.type === 'numbered-list' ? `${index + 1}. ${inner}` : `- ${inner}`
      })
      .join('\n')
  }

  const children = (node.children as CustomText[]).map(serializeInline).join('')
  switch (node.type) {
    case 'heading-one':
      return `# ${children}`
    case 'heading-two':
      return `## ${children}`
    case 'heading-three':
      return `### ${children}`
    case 'block-quote':
      return `> ${children}`
    case 'list-item':
      return `- ${children}`
    case 'paragraph':
    default:
      return children
  }
}

const EMPTY_PARAGRAPH: ParagraphElement = { type: 'paragraph', children: [{ text: '' }] }

export function deserializeFromMarkdown(markdown: string): Descendant[] {
  if (!markdown || markdown.trim() === '') return [EMPTY_PARAGRAPH]

  const nodes: CustomElement[] = []
  let listBuffer: { type: 'bulleted-list' | 'numbered-list'; items: ListItemElement[] } | null = null

  const flushList = () => {
    if (!listBuffer) return
    nodes.push({ type: listBuffer.type, children: listBuffer.items })
    listBuffer = null
  }

  for (const rawLine of markdown.split('\n')) {
    const line = rawLine.replace(/\s+$/, '')

    const bullet = /^[-*]\s+(.*)$/.exec(line)
    const numbered = /^\d+\.\s+(.*)$/.exec(line)

    if (bullet) {
      if (!listBuffer || listBuffer.type !== 'bulleted-list') {
        flushList()
        listBuffer = { type: 'bulleted-list', items: [] }
      }
      listBuffer.items.push({ type: 'list-item', children: deserializeInline(bullet[1]) })
      continue
    }
    if (numbered) {
      if (!listBuffer || listBuffer.type !== 'numbered-list') {
        flushList()
        listBuffer = { type: 'numbered-list', items: [] }
      }
      listBuffer.items.push({ type: 'list-item', children: deserializeInline(numbered[1]) })
      continue
    }

    flushList()
    if (line.startsWith('### ')) {
      nodes.push({ type: 'heading-three', children: deserializeInline(line.slice(4)) })
    } else if (line.startsWith('## ')) {
      nodes.push({ type: 'heading-two', children: deserializeInline(line.slice(3)) })
    } else if (line.startsWith('# ')) {
      nodes.push({ type: 'heading-one', children: deserializeInline(line.slice(2)) })
    } else if (line.startsWith('> ')) {
      nodes.push({ type: 'block-quote', children: deserializeInline(line.slice(2)) })
    } else if (line.trim()) {
      nodes.push({ type: 'paragraph', children: deserializeInline(line) })
    }
  }

  flushList()
  return nodes.length > 0 ? nodes : [EMPTY_PARAGRAPH]
}

// ---------------------------------------------------------------------------
// Editor command helpers
// ---------------------------------------------------------------------------

const isMarkActive = (editor: Editor, format: MarkFormat) => {
  const marks = Editor.marks(editor)
  return marks ? marks[format] === true : false
}

const toggleMark = (editor: Editor, format: MarkFormat) => {
  if (isMarkActive(editor, format)) {
    Editor.removeMark(editor, format)
  } else {
    Editor.addMark(editor, format, true)
  }
}

const isBlockActive = (editor: Editor, format: BlockFormat) => {
  const [match] = Editor.nodes(editor, {
    match: (n) => !Editor.isEditor(n) && SlateElement.isElement(n) && n.type === format,
  })
  return Boolean(match)
}

const LIST_TYPES: ReadonlyArray<BlockFormat> = ['bulleted-list', 'numbered-list']

const toggleBlock = (editor: Editor, format: BlockFormat) => {
  const isActive = isBlockActive(editor, format)
  const isList = LIST_TYPES.includes(format)

  Transforms.unwrapNodes(editor, {
    match: (n) =>
      !Editor.isEditor(n) &&
      SlateElement.isElement(n) &&
      LIST_TYPES.includes(n.type as BlockFormat),
    split: true,
  })

  Transforms.setNodes<SlateElement>(editor, {
    type: isActive ? 'paragraph' : isList ? 'list-item' : format,
  } as Partial<SlateElement>)

  if (!isActive && isList) {
    Transforms.wrapNodes(editor, { type: format, children: [] } as ListElement)
  }
}

// ---------------------------------------------------------------------------
// Toolbar
// ---------------------------------------------------------------------------

interface ToolbarButtonProps {
  isActive?: boolean
  disabled?: boolean
  onMouseDown: (e: React.MouseEvent) => void
  children: React.ReactNode
  title?: string
}

function ToolbarButton({ isActive = false, disabled = false, onMouseDown, children, title }: ToolbarButtonProps) {
  return (
    <button
      type="button"
      onMouseDown={onMouseDown}
      disabled={disabled}
      title={title}
      className={cn(
        'p-2 rounded-lg transition-all duration-150',
        isActive
          ? 'bg-indigo-500/20 text-indigo-400'
          : 'text-slate-400 hover:text-white hover:bg-slate-800',
        disabled && 'opacity-50 cursor-not-allowed'
      )}
    >
      {children}
    </button>
  )
}

function MarkButton({ format, icon: Icon, title }: { format: MarkFormat; icon: LucideIcon; title: string }) {
  const editor = useSlate()
  return (
    <ToolbarButton
      isActive={isMarkActive(editor, format)}
      onMouseDown={(e) => {
        e.preventDefault()
        toggleMark(editor, format)
      }}
      title={title}
    >
      <Icon className="h-4 w-4" />
    </ToolbarButton>
  )
}

function BlockButton({ format, icon: Icon, title }: { format: BlockFormat; icon: LucideIcon; title: string }) {
  const editor = useSlate()
  return (
    <ToolbarButton
      isActive={isBlockActive(editor, format)}
      onMouseDown={(e) => {
        e.preventDefault()
        toggleBlock(editor, format)
      }}
      title={title}
    >
      <Icon className="h-4 w-4" />
    </ToolbarButton>
  )
}

function EditorToolbar({
  onAIEnhance,
  isAILoading,
}: {
  onAIEnhance?: (type: string) => void
  isAILoading?: boolean
}) {
  return (
    <div className="flex items-center gap-1 flex-wrap">
      <div className="flex items-center gap-1">
        <MarkButton format="bold" icon={Bold} title="Bold (Ctrl+B)" />
        <MarkButton format="italic" icon={Italic} title="Italic (Ctrl+I)" />
        <MarkButton format="underline" icon={Underline} title="Underline (Ctrl+U)" />
      </div>

      <div className="w-px h-6 bg-slate-700 mx-2" />

      <div className="flex items-center gap-1">
        <BlockButton format="heading-one" icon={Heading1} title="Heading 1" />
        <BlockButton format="heading-two" icon={Heading2} title="Heading 2" />
        <BlockButton format="heading-three" icon={Heading3} title="Heading 3" />
      </div>

      <div className="w-px h-6 bg-slate-700 mx-2" />

      <div className="flex items-center gap-1">
        <BlockButton format="block-quote" icon={Quote} title="Quote" />
        <BlockButton format="bulleted-list" icon={List} title="Bullet List" />
        <BlockButton format="numbered-list" icon={ListOrdered} title="Numbered List" />
      </div>

      {onAIEnhance && (
        <>
          <div className="w-px h-6 bg-slate-700 mx-2" />
          <div className="flex items-center gap-1">
            <ToolbarButton
              onMouseDown={(e) => {
                e.preventDefault()
                onAIEnhance('improve')
              }}
              disabled={isAILoading}
              title="Enhance with AI"
            >
              {isAILoading ? (
                <Loader2 className="h-4 w-4 animate-spin text-purple-400" />
              ) : (
                <Sparkles className="h-4 w-4 text-purple-400" />
              )}
            </ToolbarButton>
          </div>
        </>
      )}
    </div>
  )
}

// ---------------------------------------------------------------------------
// Renderers
// ---------------------------------------------------------------------------

const renderElement = ({ attributes, children, element }: RenderElementProps) => {
  switch (element.type) {
    case 'heading-one':
      return (
        <h1 {...attributes} className="text-3xl font-bold mb-6 mt-8 text-white">
          {children}
        </h1>
      )
    case 'heading-two':
      return (
        <h2 {...attributes} className="text-2xl font-bold mb-4 mt-6 text-white">
          {children}
        </h2>
      )
    case 'heading-three':
      return (
        <h3 {...attributes} className="text-xl font-semibold mb-3 mt-4 text-slate-100">
          {children}
        </h3>
      )
    case 'block-quote':
      return (
        <blockquote
          {...attributes}
          className="border-l-4 border-indigo-500 pl-4 my-4 italic text-slate-300"
        >
          {children}
        </blockquote>
      )
    case 'bulleted-list':
      return (
        <ul {...attributes} className="mb-4 ml-6 list-disc">
          {children}
        </ul>
      )
    case 'numbered-list':
      return (
        <ol {...attributes} className="mb-4 ml-6 list-decimal">
          {children}
        </ol>
      )
    case 'list-item':
      return (
        <li {...attributes} className="mb-1">
          {children}
        </li>
      )
    default:
      return (
        <p {...attributes} className="mb-4 leading-relaxed">
          {children}
        </p>
      )
  }
}

const renderLeaf = ({ attributes, children, leaf }: RenderLeafProps) => {
  let rendered: React.ReactNode = children
  if (leaf.bold) rendered = <strong>{rendered}</strong>
  if (leaf.italic) rendered = <em>{rendered}</em>
  if (leaf.underline) rendered = <u>{rendered}</u>
  return <span {...attributes}>{rendered}</span>
}

// ---------------------------------------------------------------------------
// Main editor
// ---------------------------------------------------------------------------

export interface PlateEditorProps {
  initialValue?: Descendant[]
  onChange?: (value: Descendant[]) => void
  onAIEnhance?: (type: string) => void
  isAILoading?: boolean
  placeholder?: string
  readOnly?: boolean
  className?: string
}

export function PlateEditor({
  initialValue,
  onChange,
  onAIEnhance,
  isAILoading,
  placeholder = 'Start writing your story...',
  readOnly = false,
  className,
}: PlateEditorProps) {
  const editor = useMemo(() => withHistory(withReact(createEditor())), [])

  const [value, setValue] = useState<Descendant[]>(
    initialValue && initialValue.length > 0 ? initialValue : [EMPTY_PARAGRAPH]
  )

  const handleChange = useCallback(
    (newValue: Descendant[]) => {
      setValue(newValue)
      onChange?.(newValue)
    },
    [onChange]
  )

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent) => {
      if (!event.ctrlKey && !event.metaKey) return
      switch (event.key) {
        case 'b':
          event.preventDefault()
          toggleMark(editor, 'bold')
          break
        case 'i':
          event.preventDefault()
          toggleMark(editor, 'italic')
          break
        case 'u':
          event.preventDefault()
          toggleMark(editor, 'underline')
          break
      }
    },
    [editor]
  )

  return (
    <div className={cn('flex flex-col h-full', className)}>
      <Slate editor={editor} initialValue={value} onChange={handleChange}>
        {!readOnly && (
          <div className="sticky top-0 z-10 bg-slate-900/95 backdrop-blur-xl border-b border-slate-800 p-3">
            <EditorToolbar onAIEnhance={onAIEnhance} isAILoading={isAILoading} />
          </div>
        )}

        <Editable
          className={cn(
            'flex-1 px-8 py-6 outline-none',
            'text-slate-200 text-lg leading-relaxed',
            readOnly && 'cursor-default'
          )}
          style={{ fontFamily: 'Georgia, serif' }}
          placeholder={placeholder}
          readOnly={readOnly}
          renderElement={renderElement}
          renderLeaf={renderLeaf}
          onKeyDown={handleKeyDown}
          spellCheck
          autoFocus
        />
      </Slate>
    </div>
  )
}

export default PlateEditor
