create_node(kind, name, props) -> NodeHandle
create_text_node(text) -> NodeHandle

append_child(parent, child)
insert_child_before(parent, child, before)
remove_child(parent, child)

update_node(node, props)
update_text(node, text)

set_root_children(surface, children)
delete_node(node)
