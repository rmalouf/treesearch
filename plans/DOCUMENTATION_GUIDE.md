# Documentation Style Guide

**Purpose**: Defines the structure, organization, and style for all Treesearch documentation.

**Last Updated**: December 2025

---

## Documentation Structure

### Overview

Documentation consists of:

1. **README.md** - GitHub landing page with quick install and examples
2. **docs/** - Full documentation site
   - **docs/index.md** - Documentation home page
   - **User's Guide** - Topic-based guide for users
   - **API Reference** - Complete Python API documentation

### File Organization

```
treesearch/
├── README.md                          # GitHub landing page
└── docs/
    ├── index.md                       # Documentation home
    ├── guide/
    │   ├── installation.md           # Installation instructions
    │   ├── quickstart.md             # First steps tutorial
    │   ├── query-language.md         # Query syntax reference
    │   ├── reading-trees.md          # Loading treebanks
    │   ├── searching.md              # Pattern matching
    │   ├── working-with-results.md   # Navigating trees and matches
    │   └── examples.md               # Practical use cases
    └── api/
        ├── overview.md               # API organization
        ├── treebank.md               # Treebank class
        ├── tree-word.md              # Tree and Word classes
        ├── pattern.md                # Pattern class
        └── functions.md              # Standalone functions
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

### User's Guide

**Organization**: Topic-based, not linear. Readers should be able to jump to what they need.

**Structure**:

1. **Installation** - How to install from source (pip package later)
2. **Quickstart** - First working example in 5 minutes
3. **Query Language** - Complete syntax reference with linguistic examples
4. **Reading Trees** - Loading treebanks from files
5. **Searching** - Pattern matching techniques
6. **Working with Results** - Navigating trees, accessing word properties
7. **Examples** - Real-world use cases with linguistic phenomena

**Examples**:
- Use real linguistic constructions (help-to-infinitive, passive voice, relative clauses)
- Code snippets only (not full scripts - those go in separate examples/)
- Always complete and runnable within context
- Assume imports when context is clear

**External references**:
- Link to CoNLL-U format documentation (don't explain it)
- Link to Universal Dependencies for linguistic background
- Don't re-explain dependency parsing concepts

### API Reference

**Organization**: Topical grouping, not alphabetical.

**Categories**:
1. **Treebank operations** - Creating and iterating treebanks
2. **Pattern compilation** - parse_query()
3. **Searching** - search(), get_matches()
4. **Tree/Word access** - Navigating dependency structures
5. **Helper functions** - Utility functions

**For each function/class/method**:
- Full signature with types
- Brief description (1-2 sentences)
- Parameters with types and descriptions
- Return value with type and description
- At least one example
- Related functions/methods
- Notes about behavior (optional)

**Example format**:
```markdown
### get_matches()

Search one or more CoNLL-U files for pattern matches.

```python
get_matches(source: str | Path | Iterable[str | Path], query: str | Pattern, ordered: bool = True) -> Iterator[tuple[Tree, dict[str, int]]]
```

**Parameters:**
- `source` (str | Path | Iterable) - Path to file, glob pattern, or list of paths
- `query` (str | Pattern) - Query string or compiled pattern from parse_query()
- `ordered` (bool) - If True (default), return matches in deterministic order

**Returns:**
- Iterator of (tree, match) tuples

**Example:**

```python
pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')
for tree, match in treesearch.search("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
    print(f"{verb.form}: {tree.sentence_text}")
```

**See also:** search(), get_trees()
```

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
import treesearch

# Find passive constructions
pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        Aux [lemma="be"];
        V <-[aux:pass]- Aux;
    }
""")

for tree, match in treesearch.get_matches("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
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

**Purpose**: Entry point to documentation - orient readers and direct them to relevant sections.

**Required sections**:
1. Brief description (2-3 sentences)
2. Quick navigation to User's Guide sections
3. Quick navigation to API Reference sections
4. Link to installation
5. Link to GitHub/source

**What NOT to include**:
- Duplicating README content
- Feature marketing
- Extensive examples (link to guide)

**Length**: ~50-80 lines

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
import treesearch

pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')
for tree, match in treesearch.search("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
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

### User's Guide Example

```markdown
## Finding Passive Constructions

Passive constructions in Universal Dependencies typically have:
- A main verb with a passive auxiliary
- The auxiliary marked with `aux:pass`
- A passive subject marked with `nsubj:pass`

### Basic Pattern

```python
pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        Aux [lemma="be"];
        V <-[aux:pass]- Aux;
    }
""")

for tree, match in treesearch.get_matches("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
    print(tree.sentence_text)
```

### With Agent Phrases

To find passives with *by*-phrases:

```python
pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        Aux [lemma="be"];
        Agent [];
        V <-[aux:pass]- Aux;
        V -[obl:agent]-> Agent;
    }
""")
```
```

### API Reference Example

```markdown
### parse_query()

Compile a query string into a Pattern object.

```python
parse_query(query: str) -> Pattern
```

**Parameters:**
- `query` (str) - Query string in Treesearch query language

**Returns:**
- Pattern object for use with search functions

**Raises:**
- `ValueError` - If query syntax is invalid

**Example:**

```python
pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        N [upos="NOUN"];
        V -[obj]-> N;
    }
""")
```

**Notes:**
- Pattern objects are reusable and thread-safe
- Parse once and reuse for better performance

**See also:** search(), get_matches(), get_trees()
```

---

## Maintenance

### When to Update

**User's Guide:**
- New query syntax features
- Changed behavior in searching/iteration
- New usage patterns or best practices

**API Reference:**
- Any change to public API signatures
- New functions/classes/methods
- Changed return types or parameters
- New examples for clarity

**README:**
- Installation method changes
- Major feature additions (sparingly)
- Updated citation information

### Review Checklist

Before committing documentation:

- [ ] Audience appropriate (linguists who know Python, not general users)
- [ ] No marketing language
- [ ] Code examples complete and tested
- [ ] Consistent terminology (pattern/treebank/match)
- [ ] Real linguistic examples (not toy cases)
- [ ] Links working
- [ ] Proper Markdown formatting
- [ ] Appropriate section (User's Guide vs API Reference)

---

## Future Additions

As project grows:

- **Tutorials section** - Step-by-step workflows for common tasks
- **Examples repository** - Full scripts and Jupyter notebooks
- **FAQ** - Common questions and solutions
- **Troubleshooting** - Common errors and fixes

These should be separate from core User's Guide and API Reference.

---

## Version History

- **December 2025**: Initial version defining documentation structure and style
