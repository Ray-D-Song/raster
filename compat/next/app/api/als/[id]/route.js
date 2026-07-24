import { AsyncLocalStorage } from "async_hooks";

const als = new AsyncLocalStorage();

export async function GET(_request, { params }) {
  const { id } = await params;
  return als.run(id, async () => {
    // Cross await + timer so isolation depends on real async propagation.
    await Promise.resolve();
    await new Promise((resolve) => setTimeout(resolve, 15));
    await Promise.resolve();
    const store = als.getStore();
    return Response.json({ id: store });
  });
}
