import { EmbedBuilder } from "@discordjs/builders";
import type { SelectMenu } from "../../services/commands.js";
import { fetchAnilistMedia, intervalTime } from "../../utility/anilistUtil.js";

type SelectMenuData = {
    custom_id: string;
};

export const interaction: SelectMenu<SelectMenuData> = {
    custom_id: "media_selection",
    cooldown: 1,
    toggable: true,
    parse(data) {
        if (!data[0]) {
            throw new Error("Invalid Select Menu Data");
        }
        return { custom_id: data[0] };
    },
    async execute(interaction, data): Promise<void> {
        const mediaType = data.custom_id === "anime" ? "ANIME" : "MANGA";
        const media = await fetchAnilistMedia(mediaType, Number(interaction.menuValues[0]), interaction);

        if (media === null) {
            return interaction.reply({ content: "Problem trying to fetch data", ephemeral: true });
        }

        const embed = new EmbedBuilder()
            .setTitle(media.result.romaji)
            .setURL(media.result.url)
            .setImage(media.result.banner)
            .setThumbnail(media.result.cover.extraLarge)
            .setDescription(media.description.join(""))
            .setFooter({
                text: `${media.result.dataFrom === "API" ? "Displaying API data" : `Displaying cache data : expires in ${intervalTime(media.result.leftUntilExpire)}`}`,
            })
            .setColor(0x2f3136);

        try {
            await interaction.edit({ embeds: [embed] });
        } catch (error: any) {
            await interaction.reply({ embeds: [embed] });
        }
    },
};
