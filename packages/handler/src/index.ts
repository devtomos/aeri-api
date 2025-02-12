import { REST } from "@discordjs/rest";
import { getRedis } from "core";
import { env } from "core/dist/env.js";
import { HandlerClient } from "./classes/handlerClient.js";
import { Gateway } from "./gateway.js";
import { FileType, load } from "./services/commands.js";
import { registerEvents } from "./services/events.js";

const redis = await getRedis();
const rest = new REST().setToken(env.DISCORD_TOKEN);

const commands = await load(FileType.Commands);
const buttons = await load(FileType.Buttons);
const selectMenus = await load(FileType.SelectMenus);
const modals = await load(FileType.Modals);

const gateway = new Gateway({ redis, env, commands });
const client = new HandlerClient({ rest, gateway, commands, buttons, selectMenus, modals });

await gateway.connect();
await registerEvents(client);
