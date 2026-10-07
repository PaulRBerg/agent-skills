import { AnimatePresence, motion } from "motion/react";
import { useEffect, useRef } from "react";
import type { ReactNode } from "react";
import { tv } from "tailwind-variants";

import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion.js";

const valueWrapper = tv({ base: "inline-grid" });
const valueContent = tv({
  base: "col-start-1 row-start-1 -mx-0.5 rounded-xs px-0.5",
  variants: { animateChange: { true: "change-wash" } },
});

type AnimatedValueProps = {
  value: string | number | boolean | null | undefined;
  children: ReactNode;
  className?: string;
};

export function AnimatedValue({ value, children, className = "" }: AnimatedValueProps) {
  const mounted = useRef(false);

  useEffect(() => {
    mounted.current = true;
  }, []);

  const animateChange = mounted.current;
  const valueKey = value === null ? "null" : String(value ?? "undefined");

  return (
    <span className={valueWrapper({ className })}>
      <AnimatePresence initial={false} mode="popLayout">
        <motion.span
          animate={{ opacity: 1, y: 0 }}
          className={valueContent({ animateChange })}
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
