import * as React from "react";
import { Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";
import { cva, type VariantProps } from "class-variance-authority";

const loaderVariants = cva("animate-spin text-[#245442]", {
  variants: {
    size: {
      sm: "h-4 w-4",
      default: "h-6 w-6",
      lg: "h-8 w-8",
      xl: "h-12 w-12",
    },
  },
  defaultVariants: {
    size: "default",
  },
});

export interface LoaderProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof loaderVariants> {
  label?: string;
}

const Loader = React.forwardRef<HTMLDivElement, LoaderProps>(
  ({ className, size, label, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn("flex flex-col items-center justify-center gap-2", className)}
        {...props}
      >
        <Loader2 className={cn(loaderVariants({ size }))} />
        {label && <span className="text-sm text-zinc-400">{label}</span>}
      </div>
    );
  }
);
Loader.displayName = "Loader";

// Full page loader
const PageLoader = React.forwardRef<HTMLDivElement, LoaderProps>(
  ({ className, label = "加载中...", ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn(
          "fixed inset-0 z-50 flex items-center justify-center bg-zinc-950/80 backdrop-blur-sm",
          className
        )}
        {...props}
      >
        <Loader size="xl" label={label} />
      </div>
    );
  }
);
PageLoader.displayName = "PageLoader";

// Inline loader for buttons
const ButtonLoader = React.forwardRef<HTMLDivElement, Omit<LoaderProps, "size">>(
  ({ className, ...props }, ref) => {
    return (
      <div ref={ref} className={cn("inline-flex", className)} {...props}>
        <Loader2 className="h-4 w-4 animate-spin" />
      </div>
    );
  }
);
ButtonLoader.displayName = "ButtonLoader";

// Skeleton loader for content placeholders
interface SkeletonProps extends React.HTMLAttributes<HTMLDivElement> {}

const Skeleton = React.forwardRef<HTMLDivElement, SkeletonProps>(
  ({ className, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn(
          "animate-pulse rounded-md bg-zinc-800",
          className
        )}
        {...props}
      />
    );
  }
);
Skeleton.displayName = "Skeleton";

export { Loader, PageLoader, ButtonLoader, Skeleton, loaderVariants };
