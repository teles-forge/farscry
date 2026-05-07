# farscry

> See through any image. Converts screenshots into structured context
> that any AI agent can understand. No vision API. No cloud. One binary.

*farscry (n.) — a magical artifact that reveals what is hidden at a distance.*

**Problem**: Devin CLI, Claude Code, and Cursor struggle with images.
Give them a screenshot, they guess. Give them farscry output, they understand.

## Benchmark

| Tool | Time | Cost/image | Works offline |
|------|------|------------|---------------|
| **farscry** | **~80ms** | **$0** | **✅** |
| Mistral OCR | ~1.2s | $0.001 | ❌ |
| Claude Vision | ~1.8s | $0.003 | ❌ |
| Tesseract | ~300ms | $0 | ✅ |

*farscry wins on cost + agent-readiness. Tesseract wins on nothing useful for agents.*

## Install

```bash
# npm
npm install farscry

# pip
pip install farscry

# curl (any platform)
curl -fsSL https://farscry.dev/install | sh
```

## Quick start

```typescript
import { extract } from 'farscry'

const context = await extract('screenshot.png')
// → structured TOON context, ready for any agent
console.log(context)
```

## How it works

```
[image] → [binarize] → [layout detect] → [OCR per region] → [classify] → [TOON output]
```

Detects: error messages · UI fields + values · terminal output · conversations · config screens

## Output (TOON format)

TOON uses 40% fewer tokens than JSON with higher LLM accuracy.

```
screen_type: error_screenshot
error: "Payment limit exceeded"
fields:
  max_value | 1500
  status    | Active
  period    | Monthly
```

## Integrations

Works with any agent that accepts text:

- **Devin CLI**: pipe output into your message
- **Claude Code**: `farscry screenshot.png | claude`
- **Zendesk / Intercom**: inject into ticket body before AI reads it
- **MCP server**: `farscry serve --mcp`

## License

Apache 2.0
