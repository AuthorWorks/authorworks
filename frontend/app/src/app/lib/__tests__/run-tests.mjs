// Test bootstrap: resolves the repo's extensionless TS imports for node:test
// (Node 26 strips types natively; it does not add missing extensions).
// Usage: cd frontend/app && node --import ./src/app/lib/__tests__/run-tests.mjs --test <files...>
import { register } from 'node:module'
import { pathToFileURL } from 'node:url'

register(new URL('./ts-extension-hooks.mjs', import.meta.url))
