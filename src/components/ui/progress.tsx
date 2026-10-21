import * as React from "react";
import * as ProgressPrimitive from "@radix-ui/react-progress";
import { cn } from "@/lib/utils";

const Progress = React.forwardRef<
  React.ElementRef<typeof ProgressPrimitive.Root>,
  React.ComponentPropsWithoutRef<typeof ProgressPrimitive.Root> & {
    indicatorClassName?: string;
  }
>(({ className, value, indicatorClassName, ...props }, ref) => (
  <ProgressPrimitive.Root
    ref={ref}
    className={cn(
      "relative h-2 w-full overflow-hidden rounded-full bg-zinc-800",
      className
    )}
    {...props}
  >
    <ProgressPrimitive.Indicator
      className={cn(
        "h-full w-full flex-1 bg-[#245442] transition-all duration-300 ease-out",
        indicatorClassName
      )}
      style={{ transform: `translateX(-${100 - (value || 0)}%)` }}
    />
  </ProgressPrimitive.Root>
));
Progress.displayName = ProgressPrimitive.Root.displayName;

// Progress with label
interface ProgressWithLabelProps extends React.ComponentPropsWithoutRef<typeof Progress> {
  label?: string;
  showPercentage?: boolean;
}

const ProgressWithLabel = React.forwardRef<
  React.ElementRef<typeof Progress>,
  ProgressWithLabelProps
>(({ label, showPercentage = true, value, className, ...props }, ref) => (
  <div className={cn("space-y-2", className)}>
    <div className="flex items-center justify-between text-sm">
      {label && <span className="text-zinc-400">{label}</span>}
      {showPercentage && (
        <span className="text-zinc-300 font-medium">{Math.round(value || 0)}%</span>
      )}
    </div>
    <Progress ref={ref} value={value} {...props} />
  </div>
));
ProgressWithLabel.displayName = "ProgressWithLabel";

export { Progress, ProgressWithLabel };
