# Frontend Development Guidelines

This file governs the development of the SvelteKit/Svelte 5/Tailwind frontend (`src/`) for the Duplicate Finder application.

---

## Frontend Architecture

The frontend follows a layered architecture:

```text
Routes / Pages
      ↓
Components
      ↓
Services
      ↓
Tauri Commands
```

Application state is managed separately through stores.

```text
Stores
  ↑
Components
  ↑
Routes
```

### Allowed Dependencies

```text
Route → Component
Route → Service
Component → Service
Component → Store
Service → Tauri Command
```

### Not Allowed

```text
Component → invoke()
Component → Database
Store → invoke()
Store → Database
Utility → invoke()
```

Tauri IPC should always be abstracted behind service modules.

---

## Frontend Directory Structure

```text
src/
├── routes/
├── lib/
│   ├── components/
│   ├── services/
│   ├── stores/
│   ├── types/
│   ├── utils/
│   └── constants/
├── app.css
└── app.html
```

### Directory Responsibilities

#### components/

Reusable UI components.

Examples:

```text
SearchPane.svelte
SearchResultsTable.svelte
CaptureForm.svelte
Autocomplete.svelte
Modal.svelte
```

Components should focus on presentation and user interaction.

#### services/

Business logic and backend communication.

Examples:

```text
specimenService.ts
duplicateFinderService.ts
taxonomyService.ts
sessionService.ts
```

Services are responsible for:

- Calling Tauri commands
- Validation
- Workflow orchestration
- Data transformation
- Error handling

#### stores/

Shared application state.

Examples:

```text
sessionStore.ts
captureStore.ts
searchStore.ts
```

Stores are responsible only for state management.

#### utils/

Pure helper functions.

Examples:

```text
dateUtils.ts
stringUtils.ts
coordinateUtils.ts
```

Utilities should be framework-independent where possible.

#### types/

Shared TypeScript interfaces and type definitions.

Examples:

```text
Specimen.ts
Taxon.ts
DuplicateMatch.ts
```

Prefer explicit interfaces over anonymous objects.

---

## Component Design Principles

### Render-Only Principle

Components should focus on:

- Rendering UI
- Capturing user input
- Raising events
- Displaying state

Components should not:

- Perform database operations
- Execute business workflows
- Contain complex parsing logic
- Contain large data transformation routines

If logic exceeds a few lines or becomes reusable, move it into a service or utility module.

---

## State Management Rules

### Source of Truth

The database is the ultimate source of truth.

Shared application state belongs in stores.

Temporary form state belongs in components.

### Store Usage

Stores should contain:

- Current specimen
- Search results
- Session state
- User preferences
- Application status

Stores should not contain:

- Database access
- Tauri IPC calls
- Parsing logic
- Complex business rules

### Form State

Local form state should remain local until:

- Saved
- Submitted
- Explicitly promoted to shared state

Avoid duplicating store state inside components.

---

## Service Layer Rules

All backend communication must be encapsulated within services.

Preferred:

```ts
await specimenService.saveSpecimen(specimen);
```

Avoid:

```ts
await invoke("save_specimen", { specimen });
```

inside components.

Services provide a stable API between the frontend and backend.

### Service Responsibilities

Services may:

- Call Tauri commands
- Validate data
- Transform data
- Coordinate workflows
- Handle errors

Services should not:

- Manipulate DOM elements
- Render UI
- Maintain application state

---

## Component Breakdown Requirements

Every major feature should be decomposed into focused components.

### Search Column

```text
SearchPane.svelte
SearchFilters.svelte
SearchResultsTable.svelte
SearchToolbar.svelte
```

### Capture Column

```text
CaptureForm.svelte
TaxonomySection.svelte
GeographySection.svelte
DateCollectorSection.svelte
DeterminationSection.svelte
```

### Global Components

```text
Toolbar.svelte
ImportExportActions.svelte
Modal.svelte
NotificationToast.svelte
```

Avoid creating large components exceeding several hundred lines where practical.

---

## Parsing & Utility Rules

### No Inline Parsers

Parsing logic must reside in dedicated utility modules.

Examples:

```text
parseScientificName()
parseCollectorNumber()
parseVerbatimDate()
parseElevation()
```

Avoid embedding parsing routines directly in components.

### Pure Functions

Utility functions should be:

- Deterministic
- Stateless
- Easily unit-testable

Where possible:

```ts
input -> output
```

without side effects.

---

## Svelte 5 Requirements

- Use Svelte 5 syntax exclusively.
- Follow established project patterns.
- Prefer explicit state ownership.
- Keep reactive logic simple and predictable.
- Avoid unnecessary derived state.
- Prefer composition over large monolithic components.

---

## Error Handling

All service calls should handle failures gracefully.

Components should:

- Display meaningful error messages
- Avoid silent failures
- Provide user feedback during long-running operations

Unexpected errors should be surfaced through a consistent notification mechanism.

---

## Styling & Typography

### Styling

Use Tailwind CSS v4 utility classes.

Use project design tokens and CSS variables defined in `app.css`.

Avoid ad-hoc colour definitions.

### Layout

The application uses a structured, grid-based design language.

Requirements:

- `rounded-none`
- Consistent spacing
- Consistent alignment
- Strong visual hierarchy

### Interactions

Interactive elements should include:

```css
transition-all duration-200 ease-in-out
```

where appropriate.

Provide clear hover, focus, active, and disabled states.

### Typography

- Use **Outfit** for headings.
- Use **Inter** for body text.
- Maintain accessible contrast ratios.

---

## Testing Considerations

Business logic should be testable without mounting Svelte components.

Prefer:

```text
Component
    ↓
Service
    ↓
Utility
```

so that services and utilities can be tested independently using Vitest.

Avoid placing critical business logic exclusively inside component files.

---

## AI Agent Guidelines

When implementing frontend functionality:

1. Check for an existing service before creating a new one.
2. Place backend communication inside services.
3. Place shared state inside stores.
4. Place reusable logic inside utilities.
5. Keep components focused on presentation and interaction.
6. Prefer extending existing modules over creating new generic files.

Avoid creating files such as:

```text
helpers.js
misc.js
utils2.js
dbService.js
```

Prefer descriptive, domain-specific names.
