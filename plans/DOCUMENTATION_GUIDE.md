# Documentation Style Guide

**Purpose**: Defines the structure, organization, and style for all Treesearch documentation.

**Last Updated**: December 2025

---

## Documentation Structure

### Overview

Documentation consists of:

1. **README.md** - GitHub landing page with quick install and examples
2. **docs/** - Full documentation site (flat structure)
   - **docs/index.md** - Documentation home with quick start
   - **docs/tutorial.md** - Complete walkthrough from installation to advanced usage
   - **docs/query-language.md** - Query syntax reference
   - **docs/api.md** - Functions and classes reference
   - **docs/internals.md** - Architecture for contributors

### File Organization

```
treesearch/
├── README.md                          # GitHub landing page
└── docs/
    ├── index.md                       # Documentation home with quick start
    ├── tutorial.md                    # Complete walkthrough
    ├── query-language.md              # Query syntax reference
    ├── api.md                         # Functions and classes
    └── internals.md                   # Architecture for contributors
```

---

## Audience and Tone

### Target Audience

- **Primary**: Linguists and corpus linguistics researchers
- **Secondary**: Students in linguistics/NLP programs
- **Assumptions**:
  - Know Python programming
  - Understand dependency parsing (heads, dependents, deprels)
  - Familiar with corpus linguistics methods
  - Working with CoNLL-U formatted data

### Tone and Style

**Academic and straightforward**:
- Write for fellow researchers, not consumers
- No marketing language or "selling points"
- No hyperbole ("amazing", "powerful", "blazing fast")
- Focus on functionality and what it does
- Be precise and factual

**Good examples**:
- "Treesearch finds structural patterns in dependency treebanks."
- "The solver uses exhaustive search to find all matches."
- "Automatic parallel processing for multi-file treebanks."

**Bad examples**:
- "Treesearch is an amazing tool that will revolutionize your research!"
- "Lightning-fast searches across massive corpora!"
- "The most powerful treebank query system available!"

### Voice

- **User's Guide**: Second person ("you can", "your query")
- **API Reference**: Third person/descriptive ("Returns an iterator", "The pattern object")
- **README**: Mix of both, keep brief

---

## Content Guidelines

### Tutorial (tutorial.md)

**Purpose**: Complete walkthrough from installation to advanced usage.

**Structure**:
1. **Installation** - pip and from-source instructions
2. **Basic Usage** - Load, search, access results
3. **Writing Queries** - Node constraints, edges, precedence
4. **Working with Results** - Match dicts, word properties, tree navigation
5. **Examples** - Passive construction, collecting examples
6. **Performance Tips** - Compile once, ordered=False, gzip, streaming

**Style**:
- Progressive complexity (simple → advanced)
- Each section builds on previous
- Complete runnable examples
- Real linguistic constructions (passives, relative clauses)

### Query Language Reference (query-language.md)

**Purpose**: Complete syntax reference with examples.

**Structure**:
- Query structure overview
- Node constraints (table format)
- Edge constraints (positive and negative)
- Precedence operators
- Comments
- Examples of common patterns
- Common errors table

**Style**:
- Reference-focused (users look up specific syntax)
- Concise descriptions
- Table format for constraint types
- Code examples for each feature

### API Reference (api.md)

**Organization**: Functions first, then classes.

**Current API functions**:
1. `load(path)` → Treebank
2. `from_string(text)` → Treebank
3. `compile_query(query)` → Pattern
4. `trees(source, ordered=True)` → Iterator[Tree]
5. `search(source, query, ordered=True)` → Iterator[tuple[Tree, dict]]
6. `search_trees(trees, query)` → Iterator[tuple[Tree, dict]]

**Classes documented**:
1. `Treebank` - Collection with `.trees()` and `.search()` methods
2. `Tree` - Dependency tree with `.word(id)`, `sentence_text`, `metadata`
3. `Word` - All properties and navigation methods
4. `Pattern` - Compiled query (opaque)

**Format for each item**:
- Signature with types
- Brief description
- Simple example

**Note**: API docs include a Query Language Summary section linking to full reference

### Internals (internals.md)

**Purpose**: Architecture documentation for contributors.

**Content**:
- Architecture overview with component list
- Search algorithm pseudocode
- Constraint types explanation
- Parallelization diagram
- Design decisions rationale
- Source file reference table

**Audience**: Developers, not end users

### README.md

**Purpose**: GitHub landing page - get visitors oriented quickly.

**Content**:
- Brief description (1-2 sentences)
- Quick installation instructions
- 1-2 working examples
- Link to full documentation
- License and citation

**Length**: ~100-150 lines maximum

**What NOT to include**:
- Complete API documentation (link to docs)
- Extensive examples (link to docs)
- Implementation details
- Feature lists or selling points

---

## Writing Style

### Code Examples

**Snippets vs Scripts**:
- Documentation uses snippets (focused, illustrative)
- Full scripts go in examples/ directory
- Snippets should be complete enough to run in context

**Import statements**:
- Include in README examples (no context assumed)
- First example in each doc page should show imports
- Can omit in subsequent examples on same page if context clear

**Example style**:
```python
import treesearch as ts

# Find passive constructions
query = """
MATCH {
    V [upos="VERB"];
    V -[aux:pass]-> _;
    V -[nsubj:pass]-> Subj;
}
"""

for tree, match in ts.search("corpus.conllu", query):
    verb = tree.word(match["V"])
    print(f"{tree.sentence_text}")
```

**Comments**:
- Use sparingly
- Only when clarifying non-obvious behavior
- Prefer self-documenting code

### Terminology

**Consistent terms** (use these, not alternatives):
- "pattern" (not "query" or "search pattern")
- "treebank" (not "corpus" - corpus is broader)
- "match" (not "result" or "hit")
- "dependency tree" (not just "tree" when context unclear)
- "CoNLL-U file" (not "CoNLL file" - be specific)

**Variable names in examples**:
- Use linguistically meaningful names: `Verb`, `Subject`, `Auxiliary`
- Not: `x`, `y`, `node1`, `v`

### Markdown Style

**Headers**: Sentence case ("Query language" not "Query Language")

**Code blocks**: Always specify language (```python, ```bash)

**Lists**:
- Use `-` for unordered lists
- Use `1.` for ordered lists (auto-numbering)

**Emphasis**:
- **bold** for important terms on first introduction
- *italics* for linguistic forms/examples ("the word *help*")
- `code` for literals and identifiers

**Links**: Descriptive text, not raw URLs
- Good: "See [CoNLL-U format](https://universaldependencies.org/format.html)"
- Bad: "See https://universaldependencies.org/format.html"

---

## Linguistic Examples

### Use Real Phenomena

Examples should illustrate actual linguistic constructions researchers might study:

**Good examples**:
- Passive voice
- Control verbs (help-to-infinitive, try-to-infinitive)
- Relative clauses
- Causative constructions
- Subject-auxiliary inversion
- Double object constructions

**Bad examples**:
- Generic "find nouns"
- Artificial patterns with no linguistic relevance
- Toy examples that don't reflect real use cases

### Example Sentences

Use natural sentences in examples:
- Good: "She helped us to win the game."
- Bad: "The dog ran quickly."

Show real corpus output when possible, not made-up examples.

### Annotation Schemes

- Assume Universal Dependencies annotation
- Note when examples use specific deprels (e.g., `nsubj:pass`, `aux:pass`)
- Don't explain UD labels (assume familiarity or link to UD docs)

---

## README.md Specification

**Required sections**:
1. Project title and one-line description
2. Brief overview (what it does)
3. Installation (from source)
4. Quick example (1-2 code blocks)
5. Link to documentation
6. License
7. Citation

**Length**: ~100-150 lines

**Example structure**:
```markdown
# Treesearch

Pattern matching for dependency treebanks.

## Overview

Treesearch finds syntactic patterns in dependency-parsed corpora...

## Installation

[Brief install instructions]

## Quick Example

[One simple, complete example]

## Documentation

Full documentation: [link]

## License

MIT

## Citation

[BibTeX]
```

---

## docs/index.md Specification

**Purpose**: Entry point to documentation with quick start example.

**Current sections**:
1. Brief description (1 sentence)
2. Quick Start (install + example)
3. Features list (concise)
4. Documentation links
5. License

**What NOT to include**:
- Extensive examples (link to tutorial)
- Full API details (link to api.md)
- Implementation details (link to internals.md)

**Length**: ~50 lines

---

## Common Mistakes to Avoid

### ❌ Marketing Language

**Bad**: "Treesearch provides powerful, lightning-fast pattern matching that will revolutionize your corpus research!"

**Good**: "Treesearch finds syntactic patterns in dependency treebanks using exhaustive constraint satisfaction search."

### ❌ Incomplete Examples

**Bad**:
```python
# Find verbs
for match in search(...):
    print(match)
```

**Good**:

```python
import treesearch as ts

for tree, match in ts.search("corpus.conllu", 'MATCH { V [upos="VERB"]; }'):
    verb = tree.word(match["V"])
    print(f"Found: {verb.form}")
```

### ❌ Over-Explaining Python

**Bad**: "The for loop iterates over each match returned by the get_matches() function, which returns an iterator..."

**Good**: "Iterate over matches to process each one..."

### ❌ Wrong Terminology

**Bad**: "The query returns results from the corpus..."

**Good**: "The pattern returns matches from the treebank..."

### ❌ Vague Headers

**Bad**: "Using the API", "Advanced features"

**Good**: "Searching multiple files", "Negative constraints"

---

## Examples Template

### Tutorial Example

```markdown
## Finding Passive Constructions

Passive constructions in Universal Dependencies typically have:
- A main verb with a passive auxiliary
- The auxiliary marked with `aux:pass`
- A passive subject marked with `nsubj:pass`

### Basic Pattern

```python
import treesearch as ts

query = """
MATCH {
    V [upos="VERB"];
    V -[aux:pass]-> _;
    V -[nsubj:pass]-> Subj;
}
"""

for tree, match in ts.search("corpus.conllu", query):
    verb = tree.word(match["V"])
    print(tree.sentence_text)
```

### With Agent Phrases

To find passives with *by*-phrases:

```python
query = """
MATCH {
    V [upos="VERB"];
    Agent [];
    V -[aux:pass]-> _;
    V -[obl:agent]-> Agent;
}
"""
```
```

### API Reference Example

```markdown
### compile_query(query) → Pattern

Compile a query string into a reusable Pattern. Raises `ValueError` on syntax error.

```python
pattern = ts.compile_query('MATCH { V [upos="VERB"]; }')
```
```

---

## Maintenance

### When to Update

**tutorial.md:**
- New query syntax features
- Changed behavior in searching/iteration
- New usage patterns or best practices

**query-language.md:**
- New constraint types
- Changed syntax
- New operator support

**api.md:**
- Any change to public API signatures
- New functions/classes/methods
- Changed return types or parameters

**internals.md:**
- Architecture changes
- New components
- Changed design decisions

**README:**
- Installation method changes
- Major feature additions (sparingly)
- Updated citation information

### Review Checklist

Before committing documentation:

- [ ] Audience appropriate (linguists who know Python, not general users)
- [ ] No marketing language
- [ ] Code examples complete and tested
- [ ] Uses current API names (`compile_query`, `tree.word`, etc.)
- [ ] Consistent terminology (pattern/treebank/match)
- [ ] Real linguistic examples (not toy cases)
- [ ] Links working
- [ ] Proper Markdown formatting

---

## Future Additions

Potential additions as project grows:

- **More examples in tutorial.md** - Additional linguistic constructions
- **Examples repository** - Full scripts and Jupyter notebooks in `examples/`
- **FAQ section** - Common questions and solutions
- **Troubleshooting** - Common errors and fixes

---

## Version History

- **December 2025**: Initial version defining documentation structure and style
- **December 2025**: Consolidated to flat structure (index, tutorial, query-language, api, internals)
