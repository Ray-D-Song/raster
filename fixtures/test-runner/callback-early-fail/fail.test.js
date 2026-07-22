// These take a `done` parameter so the runner uses the callback branch.
// Returning a rejected promise / throwing before calling done is the
// scenario where the old implementation could leave the timeout alive.
it("callback promise reject fails immediately", (done) => {
  return Promise.reject(new Error("callback-promise-reject"));
}, 5000);

it("callback sync throw fails immediately", (done) => {
  throw new Error("callback-sync-throw");
}, 5000);

it("done(error) fails immediately", (done) => {
  done(new Error("done-error"));
}, 5000);
