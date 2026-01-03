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
    Underline
} from 'lucide-react'
import { useCallback, useMemo, useState } from 'react'
import { BaseEditor, createEditor, Descendant, Editor, Element as SlateElement, Text, Transforms } from 'slate'
import { HistoryEditor, withHistory } from 'slate-history'
import { Editable, ReactEditor, Slate, useSlate, withReact } from 'slate-react'
import { cn } from '../../lib/utils'

// Types
type CustomText = {
  text: string
  bold?: boolean
  italic?: boolean
  underline?: boolean
}

type ParagraphElement = { type: 'paragraph'; children: CustomText[] }
type HeadingElement = { type: 'heading-one' | 'heading-two' | 'heading-three'; children: CustomText[] }
type BlockquoteElement = { type: 'block-quote'; children: CustomText[] }
type ListElement = { type: 'bulleted-list' | 'numbered-list'; children: ListItemElement[] }
type ListItemElement = { type: 'list-item'; children: CustomText[] }

type CustomElement = ParagraphElement | HeadingElement | BlockquoteElement | ListElement | ListItemElement
type CustomEditor = BaseEditor & ReactEditor & HistoryEditor

declare module 'slate' {
  interface CustomTypes {
    Editor: CustomEditor
    Element: CustomElement
    Text: CustomText
  }
}

// Serialization helpers
export function serializeToMarkdown(nodes: Descendant[]): string {
  return nodes.map(node => serializeNode(node)).join('\n\n')
}

function serializeNode(node: Descendant): string {
  if (Text.isText(node)) {
    let text = node.text
    if (node.bold) text = `**${text}**`
    if (node.italic) text = `*${text}*`
    if (node.underline) text = `<u>${text}</u>`
    return text
  }

  const children = node.children.map(serializeNode).join('')

  switch (node.type) {
    case 'heading-one': return `# ${children}`
    case 'heading-two': return `## ${children}`
    case 'heading-three': return `### ${children}`
    case 'block-quote': return `> ${children}`
    case 'bulleted-list': return children
    case 'numbered-list': return children
    case 'list-item': return `- ${children}`
    case 'paragraph':
    default: return children
  }
}

export function deserializeFromMarkdown(markdown: string): Descendant[] {
  if (!markdown || markdown.trim() === '') {
    return [{ type: 'paragraph', children: [{ text: '' }] }]
  }

  const lines = markdown.split('\n')
  const nodes: Descendant[] = []

  for (const line of lines) {
    if (line.startsWith('# ')) {
      nodes.push({ type: 'heading-one', children: [{ text: line.slice(2) }] })
    } else if (line.startsWith('## ')) {
      nodes.push({ type: 'heading-two', children: [{ text: line.slice(3) }] })
    } else if (line.startsWith('### ')) {
      nodes.push({ type: 'heading-three', children: [{ text: line.slice(4) }] })
    } else if (line.startsWith('> ')) {
      nodes.push({ type: 'block-quote', children: [{ text: line.slice(2) }] })
    } else if (line.startsWith('- ')) {
      nodes.push({ type: 'paragraph', children: [{ text: line.slice(2) }] })
    } else if (line.trim()) {
      nodes.push({ type: 'paragraph', children: [{ text: line }] })
    }
  }

  return nodes.length > 0 ? nodes : [{ type: 'paragraph', children: [{ text: '' }] }]
}

// Helper functions
const isMarkActive = (editor: Editor, format: keyof Omit<CustomText, 'text'>) => {
  const marks = Editor.marks(editor)
  return marks ? marks[format] === true : false
}

const toggleMark = (editor: Editor, format: keyof Omit<CustomText, 'text'>) => {
  const isActive = isMarkActive(editor, format)
  if (isActive) {
    Editor.removeMark(editor, format)
  } else {
    Editor.addMark(editor, format, true)
  }
}

const isBlockActive = (editor: Editor, format: string) => {
  const nodes = Array.from(
    Editor.nodes(editor, {
      match: n => !Editor.isEditor(n) && SlateElement.isElement(n) && n.type === format,
    })
  )
  return nodes.length > 0
}

const toggleBlock = (editor: Editor, format: string) => {
  const isActive = isBlockActive(editor, format)
  const isList = format === 'bulleted-list' || format === 'numbered-list'

  Transforms.unwrapNodes(editor, {
    match: n => !Editor.isEditor(n) && SlateElement.isElement(n) &&
      (n.type === 'bulleted-list' || n.type === 'numbered-list'),
    split: true,
  })

  const newType = isActive ? 'paragraph' : isList ? 'list-item' : format
  Transforms.setNodes(editor, { type: newType as any })

  if (!isActive && isList) {
    const block = { type: format, children: [] }
    Transforms.wrapNodes(editor, block as any)
  }
}

// Toolbar Button
function ToolbarButton({
  isActive = false,
  disabled = false,
  onMouseDown,
  children,
  title,
}: {
  isActive?: boolean
  disabled?: boolean
  onMouseDown: (e: React.MouseEvent) => void
  children: React.ReactNode
  title?: string
}) {
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

// Mark Button
function MarkButton({ format, icon: Icon, title }: { format: keyof Omit<CustomText, 'text'>; icon: any; title: string }) {
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

// Block Button
function BlockButton({ format, icon: Icon, title }: { format: string; icon: any; title: string }) {
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

// Toolbar
function EditorToolbar({
  onAIEnhance,
  isAILoading,
}: {
  onAIEnhance?: (type: string) => void
  isAILoading?: boolean
}) {
  return (
    <div className="flex items-center gap-1 flex-wrap">
      {/* Text formatting */}
      <div className="flex items-center gap-1">
        <MarkButton format="bold" icon={Bold} title="Bold (Ctrl+B)" />
        <MarkButton format="italic" icon={Italic} title="Italic (Ctrl+I)" />
        <MarkButton format="underline" icon={Underline} title="Underline (Ctrl+U)" />
      </div>

      <div className="w-px h-6 bg-slate-700 mx-2" />

      {/* Headings */}
      <div className="flex items-center gap-1">
        <BlockButton format="heading-one" icon={Heading1} title="Heading 1" />
        <BlockButton format="heading-two" icon={Heading2} title="Heading 2" />
        <BlockButton format="heading-three" icon={Heading3} title="Heading 3" />
      </div>

      <div className="w-px h-6 bg-slate-700 mx-2" />

      {/* Block elements */}
      <div className="flex items-center gap-1">
        <BlockButton format="block-quote" icon={Quote} title="Quote" />
        <BlockButton format="bulleted-list" icon={List} title="Bullet List" />
        <BlockButton format="numbered-list" icon={ListOrdered} title="Numbered List" />
      </div>

      {/* AI Enhancement */}
      {onAIEnhance && (
        <>
          <div className="w-px h-6 bg-slate-700 mx-2" />
          <div className="flex items-center gap-1">
            <ToolbarButton
              onMouseDown={(e) => {
                e.preventDefault()
                onAIEnhance('style')
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

// Custom element renderer
const renderElement = (props: any) => {
  const { attributes, children, element } = props
  switch (element.type) {
    case 'heading-one':
      return <h1 {...attributes} className="text-3xl font-bold mb-6 mt-8 text-white">{children}</h1>
    case 'heading-two':
      return <h2 {...attributes} className="text-2xl font-bold mb-4 mt-6 text-white">{children}</h2>
    case 'heading-three':
      return <h3 {...attributes} className="text-xl font-semibold mb-3 mt-4 text-slate-100">{children}</h3>
    case 'block-quote':
      return <blockquote {...attributes} className="border-l-4 border-indigo-500 pl-4 my-4 italic text-slate-300">{children}</blockquote>
    case 'bulleted-list':
      return <ul {...attributes} className="mb-4 ml-6 list-disc">{children}</ul>
    case 'numbered-list':
      return <ol {...attributes} className="mb-4 ml-6 list-decimal">{children}</ol>
    case 'list-item':
      return <li {...attributes} className="mb-1">{children}</li>
    default:
      return <p {...attributes} className="mb-4 leading-relaxed">{children}</p>
  }
}

// Custom leaf renderer
const renderLeaf = (props: any) => {
  let { attributes, children, leaf } = props
  if (leaf.bold) {
    children = <strong>{children}</strong>
  }
  if (leaf.italic) {
    children = <em>{children}</em>
  }
  if (leaf.underline) {
    children = <u>{children}</u>
  }
  return <span {...attributes}>{children}</span>
}

// Main Editor Component
interface PlateEditorProps {
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
    initialValue || [{ type: 'paragraph', children: [{ text: '' }] }]
  )

  const handleChange = useCallback((newValue: Descendant[]) => {
    setValue(newValue)
    onChange?.(newValue)
  }, [onChange])

  // Handle keyboard shortcuts
  const handleKeyDown = useCallback((event: React.KeyboardEvent) => {
    if (!event.ctrlKey && !event.metaKey) return

    switch (event.key) {
      case 'b': {
        event.preventDefault()
        toggleMark(editor, 'bold')
        break
      }
      case 'i': {
        event.preventDefault()
        toggleMark(editor, 'italic')
        break
      }
      case 'u': {
        event.preventDefault()
        toggleMark(editor, 'underline')
        break
      }
    }
  }, [editor])

  return (
    <div className={cn('flex flex-col h-full', className)}>
      <Slate editor={editor} initialValue={value} onChange={handleChange}>
        {!readOnly && (
          <div className="sticky top-0 z-10 bg-slate-900/95 backdrop-blur-xl border-b border-slate-800 p-3">
            <EditorToolbar
              onAIEnhance={onAIEnhance}
              isAILoading={isAILoading}
            />
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
