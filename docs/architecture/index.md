# Architecture

What bones is, how its parts divide the work, and why the boundaries fall where they do. Read this chapter to understand the system; read [design/](../design/) when you need the behaviour of one subsystem in detail.

## The chapter

| Page | Answers |
| --- | --- |
| [overview.md](overview.md) | What are the parts, and which tier does each belong to? |
| [structure.md](structure.md) | Where does the code live, and what may depend on what? |
| [messaging.md](messaging.md) | How do the parts talk, and what does the engine guarantee about it? |
| [upgrading.md](upgrading.md) | How does a shipped application replace itself with a newer one? |

Decisions are recorded once, in [adr/](../adr/); [adr/index.md](../adr/index.md) lists them with their status. This chapter states what is true now and cites the decision that made it so — it never replays how the decision was reached.

## Two rules this documentation follows

**The documentation is the authority.** It records intent; code only exhibits behaviour, and reading intent is cheaper than reconstructing it. Where code and documentation disagree, the code is wrong. A change in behaviour updates the document first and the code after — revising a decision is expected, a document lagging behind the code is not.

**Architecture holds concepts; design holds implementation.** This chapter names components, boundaries, and guarantees. Signatures, file layout, and vocabularies live in `design/`, in crate READMEs, and in the code. The test is that an ordinary refactoring — moving, splitting, renaming — must not require a documentation change. If a page needs editing every time code moves, the page is written too low.

Between them the rules give one property worth stating plainly: a developer can read this chapter, understand the current state of the system, and propose an architectural change from it without reading the source first.
