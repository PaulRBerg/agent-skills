import { parsePort, reportStartupError, startServer } from "./server";

try {
  startServer(parsePort(process.env.AI_HANDOFFS_PORT));
} catch (error) {
  reportStartupError(error);
  process.exit(1);
}

