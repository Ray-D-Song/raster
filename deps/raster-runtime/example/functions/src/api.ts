import { randomBytes } from "node:crypto";
import { Todo } from "./react/TodoList";

const uid = () =>
  String.fromCharCode(
    ...randomBytes(20).map((d) => {
      return (d > 127 ? 97 : 65) + (d % 25);
    })
  ) + new Date().getTime();

const todos = new Map<string, Todo>();

const mapTodo = (item: Todo): Todo => ({
  ...item,
  createdDate: new Date(parseInt(item.createdDate)).toISOString(),
  completedDate: item.completedDate
    ? new Date(parseInt(item.completedDate)).toISOString()
    : null,
});

const API = {
  getAll: async () => Array.from(todos.values()).map(mapTodo),
  create: async (text: string) => {
    const newItem = {
      id: uid(),
      text,
      createdDate: Date.now().toString(),
      completedDate: null,
    };
    todos.set(newItem.id, newItem);
    return newItem;
  },
  delete: async (id: string) => {
    todos.delete(id);
  },
  update: async (todo: Omit<Todo, "createdDate">) => {
    const item = todos.get(todo.id);
    if (item) {
      item.completedDate = todo.completedDate ? Date.now().toString() : null;
      todos.set(todo.id, item);
    }
    return todo;
  },
};

export default API;
