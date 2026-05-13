farscry - Dev Status Report
**Data:** 2026-05-13
**Solicitado por:** @rust-engineer
**Fonte:** análise direta do código, git log, testes e PRD

---

1. Resumo Executivo

O projeto está **implementado em todas as camadas**, mas **não está pronto para release**.
O pipeline Rust compila, todos os 83 testes passam (zero falhas), e a arquitetura segue o PRD v0.1.0 fielmente - mas há **bloqueadores críticos** que impedem um lançamento confiável.

```
Situação geral: ~75% do caminho para o release
Testes:         83/83 passing  (3 ignored por requererem modelos reais)
Bloqueadores:   4 críticos  |  8 altos  |  6 distribuição
```

---

2. Git History - O que foi feito

```
62a8ce2  feat: add distribution pipeline - npm, pip, homebrew, curl, CI matrix
3e892ac  fix: prepare workspace for distribution
762bc4c  feat: add OCR selector and CoreML API updates
0a847f9  perf(coreml): batch=32 recognition - 22 calls -> 1, 38ms warm pipeline
e6807f2  Fix CoreML OCR recognition: 6 root-cause bugs found and fixed
c1ab7c0  Implement farscry CLI
6367e84  Implement MCP server
5d90502  Implement VASP formatters
7beee67  Implement visual diff engine
a37452b  Implement screen classifier crate
2382b43  Implement CoreML OCR crate
cc27e8b  Implement farscry core crate
4e71518  chore: initialize farscry workspace
```

13 commits. A ordem segue exatamente a dependency order das stories 1-10 do PRD §17.

---

3. Status por Crate

3.1 `farscry-core` -  Completo

**O que foi implementado:**
- `StateId([u8; 8])` - tamanho correto, Display como `phash:<16-hex>`
- `VaspOutput` com 12 campos exatos do PRD §6
- `VaspDelta` com 8 campos + `DeltaEntry` enum (Appeared/Removed/Changed/Unchanged)
- `BatchResult { path: PathBuf, output: Result<VaspOutput, FarscryError> }`
- `FarscryError::LanguageNotInstalled(String)` com exit code 3
- `Pipeline` com `Arc<dyn Trait + Send + Sync + 'static>` (correto para rayon)
- `process_batch` com rayon (lazy decode por worker - sem pico Nx8MB)
- pHash 32x32 2D-DCT completo com `rustdct` (não FFTW, não 8x8 block-DCT)

**Alteração em relação ao design:**
- ~~`HocrOutput`~~ -> `OcrOutput` (nome final do PRD, não "hOCR")
- `ClassifiedScreen` adicionado como tipo intermediário não especificado originalmente (necessário para `VaspFormatter` trait)

**Testes:** 16 passing - inclui estabilidade perceptual 1px, sensibilidade a error banner, determinismo 100 runs.

---

3.2 `farscry-ocr-coreml` -  Completo (com desvio arquitetural intencional)

**O que foi implementado:**
- Backend CoreML nativo com `objc2-core-ml` (NÃO `oar-ocr`)
- Pipeline: detection (DBNet++) -> postprocess DB -> recognition (SVTR-LCNet, batch=32)
- Normalização correta: ITU-R BT.601, range [0,1], std=[0.5,0.5,0.5]
- `mlarray_to_ndarray` com stride real (resolve bug de padding 64-byte alinhamento)
- `run_recognition_batch`: único call CoreML para até 32 crops (batch=32 fixo no modelo)
- `ensure_compiled` + `verify_models` (SHA256 antes do carregamento)
- `from_models_dir` e `from_model_paths` (testável sem models reais)

**Desvio arquitetural - INTENCIONAL E CORRETO:**

O doc de arquitetura original dizia usar `Arc<Mutex<CoreMlOcr>>`. Na prática, `MLModel`
não é `Send + Sync` pelo Objective-C runtime - não pode ser wrapped em `Arc<Mutex>` de
forma segura. A implementação adotou o **dedicated-thread pattern** correto:

```rust

pub struct CoreMlOcrEngine {
    sender: SyncSender<OcrRequest>,  // envia work para thread dedicada
    _handle: JoinHandle<()>,          // thread que detém MLModel exclusivamente
}


```

Essa mudança resolve o problema de raiz. A thread dedicada detém os modelos e processa
requests serialmente via channel.

**Outro desvio:**
- O doc dizia usar `msg_send![MLModel::class(), compileModelAtURL:toURL:error:]`. A
  implementação usa `crate::model::ensure_compiled` com a API disponível nos bindings atuais.

**Testes:** 4 passing, 1 ignored (integration que precisa de modelos reais).

---

3.3 `farscry-ocr-ort` -  Completo

**O que foi implementado:**
- Todas as otimizações A+B+C+D+F da arquitetura:
  - **A:** `with_intra_threads(physical_cores)` - `(logical / 2).max(1)` em x86
  - **B:** `with_optimization_level(Level2)` - não Level3 (evita regressão Intel)
  - **C:** `limit_side_len: Some(640)` - det mais rápido
  - **D:** `region_batch_size(32)`
  - **F:** modelo English default (`en_pp-ocrv5_mobile_rec.onnx`)
- `verify_models` com SHA256 antes do carregamento
- `from_models_dir` / `new` como API pública

**Testes:** 5 passing, 1 ignored (integration com modelos).

**Bug existente (não crítico para agora):**
```rust

TextRegion {
    w: 100.0, // Default estimate - TODO: calculate from bbox
    h: 20.0,  // Default estimate - TODO: calculate from bbox
}
```
Os tamanhos w/h dos TextRegions vêm hardcoded em vez de serem calculados
do bounding box real. Impacto: affordances terão coordenadas w/h imprecisas.
Não quebra OCR - apenas a precisão das dimensões de elemento.

---

3.4 `farscry-ocr` (selector) -  Completo

```rust


pub use farscry_ocr_coreml::CoreMlOcrEngine as PlatformOcrEngine;


pub use farscry_ocr_ort::OrtOcrEngine as PlatformOcrEngine;
```

**Desvio do design:**
O doc especificava módulos `coreml.rs` e `ort.rs` dentro de `farscry-ocr`. A implementação
usa re-exports diretos com `cfg` em `lib.rs`. Funcionalmente idêntico, mais simples.

**Testes:** 1 passing.

---

3.5 `farscry-classifier` -  Completo

**O que foi implementado:**
- Detecção por prioridade: Terminal -> Config -> Conversation -> Error -> Ui
- Terminal: `$`, `#`, `%`, `>>>`, `Traceback`, `File "`, `at line`, `Error:` standalone
- Config: >= 2 regiões terminando em `:`
- Conversation: >= 40% das regiões com 1-3 palavras
- Error: qualquer região contendo `error` ou `exception` (case-insensitive)
- Classificação de elementos por screen_type (Label, Button, Input, Heading, Error)
- `AffordanceAction::Select` presente (fix do bloqueador PO X-7)

**Benchmark incluso** (bench.rs): classifica 20 e 50 elementos para medir performance.

**Desvio da spec original:**
A spec citava "EfficientNet-B0 INT8 via ort -> TypedUiTree" (ML-based classifier).
A implementação é **rule-based** (sem modelo ML). Isso foi uma decisão intencional e
validada pelo PRD final - o spike provou 89.4% OOD accuracy com regras, suficiente
para v0.1.0. O PRD foi atualizado: mínimo é 85% (não 92% como draft inicial).

**Testes:** 27 passing.

---

3.6 `farscry-diff` -  Completo

**O que foi implementado:**
- Context gate: `similarity < 0.20` -> retorna delta vazio imediatamente
- Pass 1 (rough): text-only matching (threshold 0.70, greedy)
- Scroll offset: mediana de deslocamentos (dx, dy) sobre matches do Pass 1
- Pass 2 (full): bipartite matching com score = 0.4xtext + 0.4xposition + 0.2xtype
- Gaussian position proximity: `exp(-dist² / (2 x 80² ))` pixels
- Classificação: score > 0.95 -> Unchanged; else -> Changed; não-matched -> Removed/Appeared
- Levenshtein normalizado para text similarity

**Desvio:** `tokens_saved` sempre retorna `None` - requer dimensões de imagem que
não estão disponíveis na camada de diff. Um `TODO` está marcado. Não é bloqueador
de funcionalidade, mas é **gap visível no output** ("Token savings: ~0").

**Testes:** 10 passing - inclui scroll detection (test 1), field filled (test 2),
error appeared (test 3), context gate (test 4), token savings stub (test 5).

---

3.7 `farscry-formatter` -  Completo

**O que foi implementado:**
- `format_vasp`: header VASP + elementos ordenados por cy->cx + affordances + token savings
- `format_json`: serde_json pretty/compact
- `format_diff`: delta formatado
- `format_batch`: múltiplas imagens com separador `---`
- Position labels: `[top-left]`, `[middle-center]`, `[bottom-right]` (grid 3x3)
- Token savings: fórmula cloud vision systems `ceil(w/512) * ceil(h/512) * 170 + 85`
- `generate_agent_context`: one-liner por screen_type

**Desvio do PRD:**
PRD §6 mostra `[mid-left]` mas a implementação usa `[middle-left]`. Isso foi um
bloqueador PO (X-5). A implementação escolheu `[middle-*]` que é mais legível.
Impacto: cosmético. Não quebra nada, mas difere da spec publicada.

**Testes:** 11 passing.

---

3.8 `farscry-mcp` -  Implementado mas NÃO compatível com MCP real

**O que foi implementado:**
- UDS server (`~/.farscry/mcp.sock`) e TCP server (`127.0.0.1:<port>`)
- State tracking (`last_state`) para auto-diff
- `tokio::task::spawn_blocking` para inference (correto - nunca segura Mutex sobre `.await`)
- `farscry_extract` e `farscry_diff` como métodos JSON-RPC
- Trait `PipelineOps` (testável com `MockPipeline`)

**BLOQUEADOR CRÍTICO - Protocolo MCP:**
O servidor implementa JSON-RPC simples, mas o **protocolo MCP real** exige:
1. Handshake `initialize` / `initialized`
2. `tools/list` para discovery
3. `tools/call` como método canônico (não `farscry_extract` diretamente)
4. Streaming de progresso via `notifications/message`

Um MCP client real (ex: cloud model Desktop, MCP client) **não vai conseguir se conectar**.
O servidor responde métodos desconhecidos com erro, mas nunca implementa o handshake.

**Outro problema:**
`format_vasp_text` em `mcp/src/lib.rs` usa `{:?}` (Debug fmt) para element_type e
position, produzindo output como `[Label] 100.0 "Save"` em vez do formato VASP.
O formatter correto (`farscry_formatter::VaspFormatter`) não é usado pelo MCP.

**Testes:** 9 passing (todos com MockPipeline - não testam MCP protocol compliance).

---

3.9 `farscry` (binary) -  Implementado,  gaps menores

**O que foi implementado:**
- Todos os subcomandos: `extract`, `diff`, `serve`, `install-lang`, `setup`
- `--from-clipboard` macOS (via AppleScript + osascript)
- `--from-clipboard` Linux (via xclip/wl-paste)
- stdin pipe (`cat img.png | farscry`)
- `--json`, `--context`, `--lang`, `--max-size`
- Validação: magic bytes (PNG/JPG/WEBP/GIF), file size, dimensões mínimas 50px
- Exit codes: 1 (input), 2 (OCR), 3 (language not installed) - mapeamento correto
- `OnceLock<Arc<Pipeline>>` - pipeline construído uma vez, reutilizado
- `FarscryPipelineAdapter` - wires Pipeline real ao McpServer
- `resolve_models_dir()`: env var > ~/.farscry/models > exe dir > spike/models

**Flags NÃO implementadas (gap vs PRD §11):**

| Flag PRD | Status | Impacto |
|---|---|---|
| `-o context.vasp` | No ausente | Launch criteria item |
| `--text-only` | No ausente | PRD §11 |
| `farscry diff --agent` | No ausente | PRD §11 |
| `--affordances` |  aceito mas ignorado | Não muda output |

**Bug stdout:**
`farscry-ocr-coreml/src/engine.rs` linha 212:
```rust
println!("OCR pipeline completed in {:.2}ms", ...);  // <- stdout! viola VASP spec
```
PRD §10: *"stdout is ALWAYS clean. VASP/JSON to stdout only."*
Deve ser `eprintln!`.

**Testes:** 0 testes unitários no crate `farscry` binário.

---

3.10 Distribution -  Pipeline pronto,  não executado

**O que foi criado:**
- CI: audit + fmt + clippy + check + test (3 plataformas: ubuntu, macos, windows)
- Release: 4 plataformas (aarch64-apple-darwin, x86_64-apple-darwin, x86_64-linux-gnu, x86_64-windows-msvc)
- Bundling ORT dylib com fix de rpath (macOS: `install_name_tool`, Linux: `patchelf`)
- SHA256 por binário (da distribuição npm/pip)
- npm postinstall.js (download + SHA256 verify + permissão +x)
- pip hatch_build.py (mesmo pattern)
- Homebrew formula
- install.sh (curl | sh)
- publish-npm, publish-pypi, publish-crates jobs em sequência após build

**O que NÃO existe ainda:**
- Secrets `NPM_TOKEN`, `PYPI_TOKEN`, `CARGO_REGISTRY_TOKEN` a serem configurados no GitHub
- SHA256 reais dos modelos ONNX nas constantes `verify.rs` (atualmente `todo!()` ou hashes falsos)
- Nenhuma release foi publicada - zero versions no npm/PyPI/crates.io

---

4. Desvios do Design Original - Lista Completa

| # | Componente | Design Original | Implementado | Tipo |
|---|---|---|---|---|
| D1 | `farscry-preprocessor` | Crate separada no workspace | `IdentityPreprocessor` inline em `main.rs` | Simplificação |
| D2 | CoreML threading | `Arc<Mutex<CoreMlOcr>>` | Dedicated-thread + channel | Correção (MLModel não é Send) |
| D3 | MCP protocol | Protocolo MCP completo (initialize + tools/list + tools/call) | JSON-RPC simples com métodos diretos | **Bloqueador** |
| D4 | Classifier | EfficientNet-B0 INT8 via ORT | Rule-based (keyword + geometry) | Deliberado (validado no spike) |
| D5 | Position labels | `[mid-left]` | `[middle-left]` | Cosmético |
| D6 | `tokens_saved` | Calcula economia real | Sempre `None` (TODO) | Gap menor |
| D7 | `--install-lang` | Download real do CDN + SHA256 | Stub -> erro -> v0.2.0 | Scope cut |
| D8 | `-o` flag | Output para arquivo | Ausente | Gap |
| D9 | `--text-only` | Flag implementada | Ausente | Gap |
| D10 | `diff --agent` | Compact delta format | Ausente | Gap |
| D11 | `farscry-ocr` modules | `coreml.rs` + `ort.rs` módulos | `cfg` + `pub use` re-exports | Simplificação |
| D12 | OCR latency log | stderr | CoreML imprime em stdout | **Bug** |
| D13 | MCP formatter | Usa `farscry-formatter` | Duplica formatter com Debug fmt | Bug |
| D14 | ORT TextRegion w/h | Calculado do bbox | Hardcoded 100.0/20.0 | Gap de precisão |
| D15 | `--affordances` flag | Filtra affordances no output | Aceita mas ignora parâmetro | Gap |

---

5. Status dos Testes

Resultado Total

```
Total: 83 testes passando | 0 falhando | 3 ignorados
```

Por Crate

| Crate | Passando | Ignorados | Falhando |
|---|---|---|---|
| `farscry-classifier` | 27 | 0 | 0 |
| `farscry-core` | 16 | 0 | 0 |
| `farscry-diff` | 10 | 0 | 0 |
| `farscry-formatter` | 11 | 0 | 0 |
| `farscry-mcp` | 9 | 0 | 0 |
| `farscry-ocr` | 1 | 0 | 0 |
| `farscry-ocr-coreml` | 4 | 1 | 0 |
| `farscry-ocr-ort` | 5 | 1 | 0 |
| `farscry` (bin) | 0 | 0 | 0 |
| **Total** | **83** | **3** | **0** |

Testes Ignorados

Ambos requerem arquivos de modelo reais (`spike/models/`):
- `farscry_ocr_coreml::engine::tests::test_integration_with_actual_models`
- `farscry_ocr_ort::engine::tests::test_integration_with_actual_models`

Para rodar: `cargo test -p farscry-ocr-coreml -- --ignored --nocapture`

O que NÃO tem cobertura de teste

- End-to-end: nenhum teste verifica OCR -> classifier -> formatter -> output real
- CLI binary: nenhum teste de integração do comando `farscry screenshot.png`
- MCP protocol compliance: testes existentes usam MockPipeline, não testam handshake real
- Benchmark de performance: a suite spike existe mas não está integrada ao `cargo test`

---

6. Benchmark Publicado (spike)

```json
{
  "n_screenshots": 20,
  "run_a_accuracy": 1.0,   // farscry: 100%
  "run_b_accuracy": 1.0,   // cloud vision: 100%
  "avg_token_reduction_x": 3.7,
  "avg_farscry_ocr_ms": 453.3   // <- dev build sem otimização
}
```

O 453ms é build de desenvolvimento. O CoreML release build medido no spike: **21ms** (steady-state).

---

7. O Que Falta para o Release

Bloqueadores Críticos ( MUST FIX antes de qualquer release)

**C1 - MCP protocol não compatível**
O servidor MCP não implementa o protocolo MCP real. Nenhum MCP client vai conseguir
se conectar. Precisa implementar `initialize`, `tools/list`, `tools/call` conforme
MCP spec. Estimativa: 4-8h.

**C2 - `println!` em stdout no CoreML engine**
`crates/farscry-ocr-coreml/src/engine.rs:212` imprime para stdout - viola contrato VASP.
Fix trivial: trocar `println!` por `eprintln!`. 5 minutos.

**C3 - SHA256 dos modelos ONNX ausentes**
`crates/farscry-ocr-ort/src/verify.rs` e `crates/farscry-ocr-coreml/src/verify.rs`
precisam ter as hashes SHA256 reais dos arquivos de modelo. Sem isso, `verify_models()`
não protege contra adulteração (falha ou não verifica nada). Estimativa: 30min
(calcular hashes + inserir nas constantes).

**C4 - Nenhum teste de integração end-to-end**
Os 83 testes existentes são todos unitários com mocks. Antes do release, precisa de pelo
menos 1 teste end-to-end que rode OCR real -> saída VASP verificável. Estimativa: 2-4h.

Bloqueadores Altos ( SHOULD FIX antes do release público)

**A1 - `tokens_saved` sempre None**
VaspDelta sempre retorna `tokens_saved: None`. Fácil de corrigir passando dimensões
de imagem para o diff engine. 1-2h.

**A2 - MCP usa Debug fmt em vez do formatter real**
`farscry-mcp/src/lib.rs::format_vasp_text` usa `{:?}` (Debug). Deve usar
`farscry_formatter::VaspFormatter::format_vasp`. 1h.

**A3 - ORT TextRegion w/h hardcoded**
`farscry-ocr-ort/src/engine.rs:119-120` - w=100.0, h=20.0 fixos. Calcular dos pontos
do bounding box real. 1-2h.

**A4 - `-o` flag ausente**
Launch criteria §14 lista `-o context.vasp`. 1-2h.

**A5 - `--text-only` e `diff --agent` ausentes**
PRD §11 descreve ambos. 1-2h cada.

**A6 - `--affordances` ignorado**
Flag existe mas não muda o output. 30min.

**A7 - Sem testes no crate binário**
Nenhum coverage para validate_image, exit codes, subcommands. 2-4h.

**A8 - `install-lang` é stub visível**
Retorna erro de imediato. Pode ser mantido como stub para v0.2.0 mas mensagem deve
ser mais clara e exit code correto (3). 30min.

Blockers de Distribuição/Launch ( PRÉ-LAUNCH)

| # | Item | Status |
|---|---|---|
| L1 | Secrets `NPM_TOKEN`, `PYPI_TOKEN`, `CARGO_REGISTRY_TOKEN` no GitHub | No |
| L2 | SHA256 dos modelos ONNX calculados e commitados | No |
| L3 | Demo GIF gravado (< 15s, real error, sem narração) | No |
| L4 | farscry.dev atualizado com copy final + números benchmark | No |
| L5 | `cargo audit` passa sem advisories |  não verificado |
| L6 | `npm install farscry` testado nas 4 plataformas | No |

---

8. Checklist de Launch (PRD §14) - Estado Atual

```
No  cargo install farscry works           -> não publicado no crates.io
No  npm install farscry - Mac M1/M2/M3    -> não testado
No  npm install farscry - Mac Intel       -> não testado
No  npm install farscry - Linux x86_64    -> não testado
No  npm install farscry - Windows         -> não testado
No  pip install farscry - todas plataformas -> não testado
  farscry screen.png -> VASP válido      -> pipeline compila, mas sem teste e2e com modelos
  farscry diff -> delta correto          -> diff engine OK, mas sem teste e2e com modelos
  farscry --from-clipboard macOS        -> código existe, não testado em produção
  farscry --from-clipboard Linux        -> código existe, não testado em produção
No  farscry serve --mcp -> conecta MCP     -> protocolo incompleto (C1)
No  Demo GIF gravado                       -> ausente
No  farscry.dev live com copy final        -> não atualizado
  vasp-protocol.github.io/spec live     -> marcado como done no PRD
  Benchmark publicado (N=40)            -> spike existe, 100% accuracy, 3.7x tokens
  cargo audit passes                     -> não verificado nesta sessão
  NOTICES.md existe                      -> presente no repo
  SHA256 em todos os binários           -> CI gera, mas modelos sem hash real (C3)
```

---

9. Arquitetura Final Implementada vs Planejada

Planejada (PRD §7)
```
farscry/ (virtual manifest)
├── crates/
│   ├── farscry-core/        types + traits + pHash + FarscryError
│   ├── farscry-ocr/         selector: CoreML (macOS) | ORT (all)
│   ├── farscry-classifier/  screen-type router + spatial rules
│   ├── farscry-diff/        bipartite matching + context gate
│   ├── farscry-formatter/   VASP text + JSON output
│   └── farscry-mcp/         UDS MCP server + 2 tools
├── crates/farscry/          binary CLI
├── npm/                     postinstall wrapper
└── pip/                     hatch build hook wrapper
```

Implementada (atual)
```
farscry/ (virtual manifest)
├── crates/
│   ├── farscry-core/         types + traits + pHash + FarscryError + Pipeline
│   ├── farscry-ocr/          selector compile-time (cfg feature)
│   ├── farscry-ocr-coreml/   CoreML dedicada-thread pattern [ADICIONADA vs design]
│   ├── farscry-ocr-ort/      ORT com A+B+C+D+F [ADICIONADA vs design]
│   ├── farscry-classifier/   rule-based (não EfficientNet)
│   ├── farscry-diff/         bipartite + scroll + context gate
│   ├── farscry-formatter/    VASP text + JSON
│   └── farscry-mcp/          JSON-RPC simples (não protocolo MCP real)
├── crates/farscry/           CLI completo (com gaps menores)
├── npm/                      postinstall.js
├── pip/                      hatch_build.py
├── homebrew/                 Formula/farscry.rb [ADICIONADO vs PRD workspace]
├── spike/                   (não membro do workspace)
├── spike-coreml-ep/         (não membro do workspace)
└── spike-native-coreml/     (não membro do workspace)
```

Diferença principal: `farscry-preprocessor` não foi criada (o `Preprocessor` trait existe
em `farscry-core`, a implementação real é `IdentityPreprocessor` inline no binary).

---

10. Estimativa de Esforço para Release

Considerando apenas os bloqueadores críticos e altos:

| Item | Estimativa |
|---|---|
| C1 - MCP protocol completo | 4-8h |
| C2 - stdout -> stderr no CoreML | 5min |
| C3 - SHA256 dos modelos | 30min |
| C4 - Teste e2e com modelos reais | 2-4h |
| A1 - tokens_saved | 1-2h |
| A2 - MCP formatter real | 1h |
| A3 - ORT TextRegion bbox | 1-2h |
| A4/A5/A6 - flags CLI | 3-4h |
| L1-L6 - distribuição/launch | 4-8h |
| **Total estimado** | **17-30h** |

---


