import { useEffect, useRef, type ReactNode } from "react";
import { AnimatePresence, motion } from "motion/react";
import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion";

interface AnimatedValueProps {
  value: string | number | boolean | null | undefined;
  children: ReactNode;
  className?: string;
}

export function AnimatedValue({
  value,
  children,
  className = "",
}: AnimatedValueProps) {
  const mounted = useRef(false);

  useEffect(() => {
    mounted.current = true;
  }, []);

  const animateChange = mounted.current;
  const valueKey = value === null ? "null" : String(value ?? "undefined");

  return (
    <span className={`inline-grid ${className}`}>
      <AnimatePresence initial={false} mode="popLayout">
        <motion.span
          animate={{ opacity: 1, y: 0 }}
          className={`col-start-1 row-start-1 -mx-0.5 rounded-xs px-0.5 ${animateChange ? "change-wash" : ""}`}
          data-motion-item
          exit={{ opacity: 0, y: -4 }}
          initial={animateChange ? { opacity: 0, y: 4 } : false}
          key={valueKey}
          transition={{
            duration: MOTION_DURATION.field,
            ease: MOTION_EASE,
          }}
        >
          {children}
        </motion.span>
      </AnimatePresence>
    </span>
  );
}
