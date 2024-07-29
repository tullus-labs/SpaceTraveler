import type { MetaFunction } from "@remix-run/node";
import { invoke } from '@tauri-apps/api/tauri'
export const meta: MetaFunction = () => {
    return [
        { title: "New Remix App" },
        { name: "description", content: "Welcome to Remix!" },
    ];
};

export default function Index() {
    return (
        <div className="font-sans p-4">
            <button onClick={() => {
                invoke("call").then(r => {})
            }}>Call</button>
        </div>
    );
}
