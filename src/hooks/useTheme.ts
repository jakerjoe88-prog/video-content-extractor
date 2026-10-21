import { useThemeContext } from "@/components/ThemeProvider";

/**
 * Hook to access and control the theme
 * Must be used within a ThemeProvider
 */
export function useTheme() {
  const { theme, resolvedTheme, setTheme } = useThemeContext();
  return { theme, resolvedTheme, setTheme };
}

/**
 * Get the resolved theme (actual light/dark, not "system")
 * Must be used within a ThemeProvider
 */
export function useResolvedTheme(): "light" | "dark" {
  const { resolvedTheme } = useThemeContext();
  return resolvedTheme;
}
