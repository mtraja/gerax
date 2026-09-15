# Design System — Colégio Militar de Fortaleza

**Projeto:** Redesign do portal `cmf.eb.mil.br`
**Tema central:** "Casa de Eudoro Corrêa"
**Versão:** 1.0 — Draft para validação
**Data:** Setembro de 2026

---

## 1. Visão geral

Este documento define a linguagem visual e os padrões de interface para o novo
portal do Colégio Militar de Fortaleza (CMF). O design parte de dois eixos:

- **Tradição militar**: o brasão do CMF — com a jangada, o mar, o sol, o céu —
  como fonte única de cor, forma e espírito.
- **Modernidade digital**: clareza, acessibilidade, responsividade e
  componentes reutilizáveis, no padrão de serviços públicos digitais do
  governo federal (gov.br, FalaBR, LGPD).

O resultado é uma identidade própria do colégio, distinta do verde "genérico"
do Exército, sem deixar de respeitar a hierarquia institucional.

### 1.1 Objetivos

1. Dar personalidade visual única ao CMF a partir do brasão.
2. Melhorar a navegação institucional (menu, processos seletivos, comunicados).
3. Sanar débitos de acessibilidade (WCAG 2.1 AA) e uso responsivo.
4. Substituir recursos datados (emoji como ícones, listas corridas de PDFs).
5. Manter compatibilidade com o ecossistema gov.br (FalaBR, alto contraste).

---

## 2. Princípios de design

| Princípio | Descrição |
|---|---|
| **Honra ao brasão** | Cada cor, forma e textura deriva de um elemento real do brasão. Nada é adicionado sem correspondência heráldica. |
| **Braço forte, mão amiga** | O visual comunica solidez (grid, cantos chamfrados, cores institucionais) e acolhimento (ar espaçoso, tom de voz claro). |
| **Simplicidade operacional** | Componentes com um único propósito. O usuário encontra em menos de 2 cliques as prioridades: calendário, comunicados, processo seletivo e área do aluno. |
| **Acessível por padrão** | Contraste AA, foco visível, navegação por teclado e modo alto contraste próprio. |
| **Fortaleza no DNA** | A jangada, o sol do Nordeste e o verde-mar aparecem com moderação, como identidade regional, não como decoração. |

---

## 3. Marca e brasão

### 3.1 Uso correto

- O brasão é o símbolo máximo; nunca é distorcido, rotacionado, recolorido ou
  tratado com efeitos (sombra, gradiente, relevo).
- Só existem duas versões: **colorida** (sobre branco-quente `--bg`) e
  **monocromática navy** (sobre claro).
- Versão monocromática dourada é reservada ao rodapé sobre navy.

### 3.2 Área de respiro

- Mínimo de **um raio do brasão (R)** em todos os lados, medido a partir do
  contorno do símbolo — nunca do invólucro da imagem.
- Mínimo de **40 px de largura no viewport** para uso em marca d'água favicon.

### 3.3 Logotipo associado

O conjunto **brasão + wordmark "COLÉGIO MILITAR DE FORTALEZA"** segue
alinhamento à esquerda, com a frase "Casa de Eudoro Corrêa" como *tagline* em
detalhes dourados. O brasão fica à esquerda, nunca centralizado em mobile.

### 3.4 Usos proibidos

- Não colocar o brasão sobre fundos coloridos ou fotografias sem a
  sobreposição de superfície brilho especificada (item 8.3).
- Não usar o brasão como ícone decorativo repetido em fundo.
- Não misturar com outras marcas sem hierarquia definida.

---

## 4. Paleta de cores

### 4.1 Origem no brasão

Cores extraídas por análise pixel da arte oficial `cmf.png` (1393×1950):

| Elemento no brasão | Cor extraída (hex) |
|---|---|
| Azul-celeste (céu) | `#70C0F0` |
| Azul-forte | `#0078C0` |
| Vermelho | `#D82018` |
| Dourado (sol/estrelas) | `#F8C000` |
| Verde-mar (jangada) | `#387068` |
| Branco | `#F8F8F8` |

### 4.2 Tokens primários (institucionais)

| Token | Valor | Uso |
|---|---|---|
| `--navy` | `#0A3D6B` | Header, rodapé, fundos de hero, títulos |
| `--red-eb` | `#C8102E` | CTAs primários, alertas, destaques de urgência |
| `--gold` | `#E9B800` | Selos, datas, decoração, hover em texto sobre navy |
| `--jade` | `#2E6E62` | Verde-mar: tags de "ambiente", barra gov (substitui o verde genérico) |
| `--celeste` | `#5FA8D3` | Realces secundários, links sobre navy |
| `--sand` | `#FAF9F6` | Branco-quente: fundo padrão (menos brusco que o branco puro) |
| `--ink` | `#23272B` | Texto principal sobre fundos claros |

> Os valores finais foram refinados das amostras brutas do brasão para atender
> equilíbrio de matiz, saturação e contraste (seção 9).

### 4.3 Tokens semânticos

Mapeiam uso, não cor. Permite trocar tema sem reescrever componentes.

| Token | Claro | Escuro "Eudoro Corrêa" |
|---|---|---|
| `--bg` | `--sand` | `--navy` |
| `--surface` | `#FFFFFF` | `#0B4678` |
| `--surface-2` | `#EFF1F3` | `#09345C` |
| `--text` | `--ink` | `--sand` |
| `--text-muted` | `#5A6068` | `#A9C4DC` |
| `--border` | `#D8DDE3` | `#1B4E7E` |
| `--primary` | `--navy` | `--celeste` |
| `--primary-hover` | `#0D4C85` | `#74BAE3` |
| `--cta` | `--red-eb` | `#E04054` |
| `--accent` | `--gold` | `--gold` |
| `--link` | `#0B5AA2` | `#8FCEF4` |
| `--error` | `#B3261E` | `#FFB4AB` |
| `--success` | `#1E7B45` | `#8FD6AE` |
| `--focus` | `#0A3D6B` | `#F5C518` |

### 4.4 Regras de uso

- O dourado **nunca** é usado para texto de leitura contínua sobre fundo claro
  (falha de contraste); reservado a selos, datas e decoração.
- O vermelho é de destaque: reservado a CTAs e urgência. Não se pinta um card
  inteiro de vermelho.
- Fundo navy pede texto `--sand` e nunca texto `--ink`.

---

## 5. Tipografia

### 5.1 Famílias

| Papel | Família | Justificativa |
|---|---|---|
| Display/Títulos | **Sora** (ou "Schibsted Grotesk") | Display grotesca com caráter monumental, boa em contraste com o brasão; traz firmeza militar com calor humano |
| Corpo/UI | **Inter** | Neutralidade legível, suporte amplo a acentos do português, métricas estáveis em UI |
| Dados/números | **Inter** com `font-variant-numeric: tabular-nums` | Datas, notas e prazos alinhados |

*Fallback:* `'Sora', -apple-system, 'Segoe UI', Roboto, sans-serif` e
`Inter, system-ui, sans-serif`.

### 5.2 Escala tipográfica

| Nível | Tamanho | Largura | Traço | Uso |
|---|---|---|---|---|
| Display | 2.75 / 3.5 rem | 700 | −2% | Hero, capa de notícias |
| H1 | 2.25 / 2.75 rem | 700 | −1.5% | Título de página |
| H2 | 1.75 rem | 700 | −1% | Seção |
| H3 | 1.375 rem | 600 | 0 | Card, item de menu |
| Body | 1.0625 rem | 400 | 0 | Texto padrão |
| Small | 0.875 rem | 400 | 0 | Metadados, legendas |
| Overline | 0.8125 rem | 600 | +8% (maísc.) | Etiquetas seccionais |

### 5.3 Ritmo

- Interlinha: `1.5` para textos longos; `1.1–1.3` para display/títulos.
- Parágrafos separados por `0.75rem` de folga, nunca por recuo.
- Largura de linha de leitura: **62–72 caracteres**.
- Datas e números sempre em `tabular-nums` para alinhar listas de comunicados.

---

## 6. Grade e espaçamento

### 6.1 Grid

- 12 colunas em desktop, 8 em tablet, 4 em mobile.
- Container máximo **1280 px**, margens laterais mínimas de 24 px (16 px em mobile).
- Gutter: 24 px (16 px em mobile).

### 6.2 Escala de espaçamento (base 4)

| Token | px | Uso típico |
|---|---|---|
| `--sp-2` | 8 | gaps internos de chip |
| `--sp-3` | 12 | espaçamento de ícones |
| `--sp-4` | 16 | padding de inputs, gaps verticais de cards |
| `--sp-6` | 24 | gaps entre cards, margem de seção |
| `--sp-8` | 32 | padding de componentes (buttons grandes, hero) |
| `--sp-12` | 48 | espaçamento entre seções |
| `--sp-16` | 64 | espaçamento entre blocos de página |
| `--sp-24` | 96 | espaçamento de hero e rodapé alto |

### 6.3 Breakpoints

| Nome | Largura |
|---|---|
| `mobile` | < 640 px |
| `tablet` | 640–1023 px |
| `desktop` | 1024–1279 px |
| `wide` | ≥ 1280 px |

---

## 7. Formas e padrões decorativos

Derivados do brasão:

- **Cantos chamfrados**: cantos cortados de 12 px nos cards e botões grandes
  (`clip-path` ou `border-radius` misto), evocando insígnias. Nunca em
  elementos interativos pequenos (< 40 px).
- **Estrelas de cadete**: textura de estrelas de baixa opacidade (2–4%) como
  trama de fundo em seções navy, gerada por SVG repetido — nunca visível em
  áreas de leitura densa.
- **Listras diagonais**: faixa decorativa em azul/vermelho/dourado (fina,
  4–6 px) apenas como divisor entre hero e conteúdo, ou na base do header.
- **Sol nascente**: arco dourado parcial, usado apenas na marcação do hero
  ("semirradial") — associado ao nascer do sol sobre o mar do brasão.

---

## 8. Imagética

### 8.1 Fotografia

- Preferência por fotos reais do colégio: alunos em atividades, instalações,
  cerimônias, jangada/mar e paisagem de Fortaleza.
- Sem fotos stock genéricas de "militar sorrindo".
- Todas as fotos com texto-alt descritivo e nenhuma informação exclusiva em
  imagem sem equivalente textual.

### 8.2 Overlay padrão (hero)

Gradiente linear de `--navy (85%)` a `rgba(10,61,107,0.15)` da esquerda para a
direita, garantindo legibilidade de títulos e CTA sobre qualquer foto.

### 8.3 Superfície para o brasão

Quando o brasão aparece sobre superfície escura, usar um "escudo" circular de
`--surface` (43% de opacidade) com blur de 8 px atrás — preservando as cores
originais sem modificá-las.

### 8.4 Ilustração

- Vetores SVG próprios, só nas variações de cor da paleta.
- A **jangada** é usada como ícone de "Área do Aluno/Espaço Educacional" e na
  decoração do rodapé — símbolo de identidade local, com moderação.
- Proibido emoji como substituto de ícone (item 10.2).

---

## 9. Acessibilidade — contraste (WCAG 2.1 AA)

Taxas confirmadas para as combinações aprovadas:

| Combinação | Contraste | Verdict AA (texto normal) |
|---|---|---|
| `--navy` / `--sand` | 10.54 : 1 | ✔ |
| `--navy` / branco | 11.10 : 1 | ✔ |
| `--red-eb` / branco | 5.88 : 1 | ✔ |
| `--red-eb` / `--sand` | 5.59 : 1 | ✔ |
| `--gold` / `--navy` | 5.98 : 1 | ✔ |
| `--gold` / `--ink` | 8.11 : 1 | ✔ |
| `--celeste` / `--ink` | 5.75 : 1 | ✔ |
| `--jade` / branco | 5.96 : 1 | ✔ |
| `--celeste` / `--navy` | 4.24 : 1 | — só p/ textos grandes ou UI não-textual |

Combinações **proibidas** para texto: `--navy`/`--red-eb` (1.89:1),
`--gold` sobre fundos claros, `--navy`/`--ink`.

---

## 10. Iconografia

### 10.1 Estilo

- Tracado uniforme de **1,75 px**, cantos arredondados, viewBox 24×24.
- Paleta: `--navy` (padrão), `--sand` (sobre navy), `--gold` (destaque).
- Tamanho mínimo de alvo: **44×44 px**, ícone visual de **24 px**.

### 10.2 Mapa de ícones (substituindo os emoji atuais)

| Antigo (emoji) | Novo ícone SVG | Área |
|---|---|---|
| 🎓 | Capelo | Como Ingressar |
| 💰 | Finança (cifrão em círculo) | Setor Financeiro |
| 🗓️ | Calendário | Plano de Contratações, Calendário |
| 👨💼 | Servidor (tórax) | Servidor Civil |
| 🎭 | Máscaras | Programa Mecenas |
| 📑 | Documento | Requerimentos e Comunicados |
| ✅ | Selo de conferência | Lista de Documentos |

Ícone **reservado especial**: a jangada para o "Espaço Educacional".

---

## 11. Componentes

### 11.1 Botões

| Variante | Fundo | Texto | Borda | Uso |
|---|---|---|---|---|
| `btn-primary` | `--cta` | branco | — | Ação principal (ex.: "Realizar inscrição") |
| `btn-secondary` | `--navy` | `--sand` | — | Ação de apoio no mesmo contexto |
| `btn-ghost` | transparente | `--navy` | 1 px `--border` | Ações de contexto, filtros |
| `btn-gold` | `--gold` | `--ink` | — | Destaques premium (selo, decoração) |

Estados obrigatórios: `hover` (+escurece 8%), `focus` (anél `--focus` de 3 px),
`active` (translada 1 px), `disabled` (50% opacidade, cursor not-allowed).

Altura mínima 44 px para alvos de toque.

### 11.2 Cartões

- **Card de notícia**: imagem 16:9, overline temática, título H3, trecho com
  `clamp` de 3 linhas, metadata (data em selo dourado + tempo de leitura).
- **Card de serviço** (Área do Aluno): ícone em bagde `--navy`, título, texto,
  link "Acessar →". Fundo `--surface` com cantos chamfrados de 12 px e sombra
  sutil `0 1px 3px rgba(10,26,46,.08)`.
- **Card de contato/Ouvidoria**: ícone, título, descrição, botão de acesso ao
  FalaBR; semântica apropriada (denúncia recebe styling de urgência).

### 11.3 Selo de data

Badge retangular dourado (`--gold`, texto `--ink`) com dia e mês em coluna
(tipo *calendar day marker*), usado na lista de comunicados e cards de notícia.
É a assinatura visual do sistema.

### 11.4 Comunicados (feed de informativos)

Substitui a lista corrida de PDFs por um feed ordenável:

- **Badge de tipo**: `DE` (Divisão de Ensino) / `CA` (Corpo de Alunos) em
  `--jade` ou `--red-eb`, respectivamente (distinguíveis por cor + rótulo).
- Linha: selo de data + título em link + tag de anexo (PDF) + código do
  comunicado (ex.: "CA Nº 25").
- Filtros por tipo, mês e busca; ordenação por data padrão.
- PDFs abertos em nova aba, sempre com aviso de download.

### 11.5 Menu principal (mega-menu)

- 6 grupos: Institucional, Espaço Educacional, Divisão de Ensino, Acesso à
  Informação, Corpo de Alunos, Processo Seletivo.
- Header fixo com `--navy`, brasão à esquerda, busca à direita.
- Mobile: drawer deslizante com acordeão, foco preso (focus trap), fechamento
  por Esc.
- Item ativo indicado por `--gold` em sobrescrito tipográfico e um resquício
  de listra diagonal abaixo do texto.
- Separador "Área do Aluno" como botão de acesso rápido (estilo CTA).

### 11.6 Hero

- Carrossel de 3 slides (máx.) em fotografia com overlay (item 8.2).
- Hierarquia: overline dourada → título display → subtexto → CTA.
- Controles: setas + suspiros, indicadores numéricos (01/03), pausa em hover
  e `prefers-reduced-motion`.
- Em mobile, o overlay ocupa 100% da largura (nunca texto sobre foto sem
  superfície).

### 11.7 Ouvidoria

Grade de 4 cartões (Elogio, Sugestão/Reclamação, Solicitação, Denúncia) com
acesso direto ao FalaBR preservando os IDs de tip (tipo=1, 2, 3, 5).

### 11.8 Formulários

- Labels sempre visíveis (nunca placeholder como label).
- Inputs com `--border` 1 px, cantos chamfrados 8 px, estado `error` com mensagem
  textual + ícone, estado `success` verde.
- Textos de ajuda em `--text-muted`.

### 11.9 Alertas

| Tipo | Fundo | Borda esquerda | Ícone |
|---|---|---|---|
| Info | `--surface-2` | `--celeste` | ℹ SVG |
| Aviso | `--surface-2` | `--gold` | triângulo |
| Urgência | `--surface-2` | `--cta` | sinal de exclamação |

Sempre com `role="alert"` quando dinâmicos.

### 11.10 Breadcrumb, Tabs e Paginação

- Breadcrumb abaixo do header com separador de listra diagonal inclinada.
- Tabs: linha inferior `--gold` de 3 px no ativo, foco visível.
- Paginação em números com `tabular-nums`, ativo em `--navy`.

---

## 12. Temas

### 12.1 Tema Fortaleza (claro, padrão)

Fundo `--sand`, superficies brancas, títulos `--navy`. Uso de `--celeste` em
link e fundos de seção alternados. Consumo energético baixo, leitura
prolongada confortável.

### 12.2 Tema Eudoro Corrêa (escuro)

- Base `--navy`, superfícies `--surface`/`--surface-2` escuros, texto `--sand`.
- Destaques dourados, CTAs vermelho-claros, textura de estrelas sutil.
- Ativável por `prefers-color-scheme: dark`, por alternador manual (com
  persistência) ou, opcionalmente, por horário (modo noturno).

### 12.3 Tema Jangada (variante lúdica/educação)

- Aplicado a páginas dos anos iniciais e áreas acadêmicas leves.
- Mais `--celeste`, `--gold` e `--jade`; cantos chamfrados maiores (16 px),
  ilustrações da jangada e do sol; tipografia um passo maior.
- Mantém contraste AA; nunca sacrifica leitura por diversão.

### 12.4 Tema Alto Contraste (acessibilidade)

- Substitui o modo atual do site por uma variante controlada:
  - Fundo preto `#000`, textos amarelo `#F5C518` ou branco,
    links sublinhados, CTAs laranja `#FF8A3C`.
  - Desliga texturas (estrelas, diagonais) e gradientes.
  - Anel de foco amarelo 4 px.
  - Implementado por `class="theme-hc"` no `<html>`, independente do modo
    claro/escuro.

### 12.5 Tokens por tema

Cada tema redefine somente os tokens semânticos (item 4.3). Nenhum componente
referencia cor de tema diretamente.

---

## 13. Movimento

- Durações: `120 ms` (hover), `200 ms` (acordeões), `350 ms` (mega-menu/hero).
- Curvas: `cubic-bezier(0.2, 0.8, 0.2, 1)`.
- Honra a `prefers-reduced-motion: reduce` → transições instantâneas, slider
  sem autoplay.
- Nenhum elemento pisca mais de 3 vezes; nenhum movimento essencial para
  entender o conteúdo.

---

## 14. Padrões de página

### 14.1 Home

```
Header fixo (navy) → gov bar (jade)
Hero (foto + overlay navy + CTA processo seletivo) + listra diagonal
Faixa de serviços rápidos → Área do Aluno (4 cards com badges jangada)
Últimas Notícias (3 cards)
Comunicados em destaque (3 itens do feed com selo dourado)
Ouvidoria (4 cartões FalaBR)
Rodapé (navy + estrelas) — mapa do site, contatos, redes, selos gov.br/EB
```

### 14.2 Interna / notícia

Breadcrumb → título display → metadata (data-selo, autoria, tempo) → corpo em
coluna de leitura (62–72 chars) → faixa "compartilhar" → notícias relacionadas.

### 14.3 Comunicados

Cabeçalho com busca + filtros (tipo DE/CA, mês, ano) → lista em tabela de
linhas com selo de data, badge de tipo e link PDF → paginação.

### 14.4 Processo seletivo

Hero específico com prazos em destaque dourado (abas em contagem
regressiva), timeline de etapas, CTA de inscrição, FAQ em acordeão.

---

## 15. Tokens CSS (referência)

```css
:root {
  /* Core */
  --navy: #0A3D6B;
  --red-eb: #C8102E;
  --gold: #E9B800;
  --jade: #2E6E62;
  --celeste: #5FA8D3;
  --sand: #FAF9F6;
  --ink: #23272B;

  /* Semânticos (tema claro) */
  --bg: var(--sand);
  --surface: #FFFFFF;
  --surface-2: #EFF1F3;
  --text: var(--ink);
  --text-muted: #5A6068;
  --border: #D8DDE3;
  --primary: var(--navy);
  --cta: var(--red-eb);
  --accent: var(--gold);
  --link: #0B5AA2;
  --error: #B3261E;
  --success: #1E7B45;
  --focus: #0A3D6B;

  /* Espaçamento */
  --sp-2: 8px;  --sp-3: 12px; --sp-4: 16px; --sp-6: 24px;
  --sp-8: 32px; --sp-12: 48px; --sp-16: 64px; --sp-24: 96px;

  /* Forma */
  --chamfer: 12px;
  --chamfer-sm: 8px;
  --shadow-card: 0 1px 3px rgba(10, 26, 46, .08);

  /* Tipografia */
  --font-display: 'Sora', system-ui, sans-serif;
  --font-body: 'Inter', system-ui, sans-serif;

  /* Movimento */
  --dur-hover: 120ms;
  --dur-open: 350ms;
  --ease: cubic-bezier(0.2, 0.8, 0.2, 1);
}
```

Tema escuro:

```css
[data-theme="eudoro"] {
  --bg: var(--navy);
  --surface: #0B4678;
  --surface-2: #09345C;
  --text: var(--sand);
  --text-muted: #A9C4DC;
  --border: #1B4E7E;
  --primary: var(--celeste);
  --cta: #E04054;
  --link: #8FCEF4;
  --error: #FFB4AB;
  --success: #8FD6AE;
  --focus: #F5C518;
}
```

---

## 16. Acessibilidade (recapitulativo)

- WCAG 2.1 nível **AA** como barreira; AAA alvo onde viável.
- Navegação 100% por teclado com `skip link` ("Ir para o conteúdo").
- Foco visível em todos os elementos interativos (`--focus`).
- Imagens com `alt`, PDFs com título e data no link, ícones com `aria-hidden`
  + rótulos textuais.
- Formulários associados por `label for`/`aria-describedby`.
- Alto contraste e dark mode como temas de cidadãos (itens 12.2 / 12.4).

---

## 17. Governança e adoção

1. **Fase 1 — Tokens e base**: padronizar CSS custom properties (variáveis), tipografia,
   grade e tema claro/escuro.
2. **Fase 2 — Componentes**: biblioteca de componentes (botões, cards, selo de
   data, feed de comunicados, mega-menu, hero).
3. **Fase 3 — Páginas**: home, interna, comunicados, processo seletivo,
   ouvidoria.
4. **Fase 4 — Temas especiais**: Jangada e Alto Contraste.
5. **QA**: checagem de contraste automatizada (axe-core + pa11y), testes de
   teclado e leitores de tela (NVDA/Orca), revisão heráldica do brasão com o
   Cerimonial do Exército.

---

*Documento aberto a revisão. Próximos passos sugeridos: protótipo em HTML/CSS
das páginas-chave e validação com a Seção de Comunicação Social do CMF.*