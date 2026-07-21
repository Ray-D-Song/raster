export default async function PostPage({ params }) {
  const { id } = await params;
  return <main>Post {id}</main>;
}
