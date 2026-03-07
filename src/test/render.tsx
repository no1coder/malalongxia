import { render, type RenderOptions } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import type { ReactElement } from "react";

// Render helper that wraps components in MemoryRouter
export function renderWithRouter(
  ui: ReactElement,
  { initialEntries = ["/"], ...options }: RenderOptions & { initialEntries?: string[] } = {}
) {
  return render(ui, {
    wrapper: ({ children }) => (
      <MemoryRouter initialEntries={initialEntries}>{children}</MemoryRouter>
    ),
    ...options,
  });
}

export { render };
