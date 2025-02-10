import { bold, inlineCode } from "@discordjs/formatters";
import { capitalise } from "core";
import type { Routes } from "../types.js";
import type { TransformersType } from "./index.js";
import { filteredDescription, truncateAnilistDescription, truncateAnilistIfExceedsDescription } from "./util.js";

const MAX_DESCRIPTION_LENGTH = 4096;

export const staffTransformer: TransformersType[Routes.Staff] = (data) => {
    const descriptionBuilder = [
        `${inlineCode("Age               :")} ${data.age}\n`,
        `${inlineCode("Gender            :")} ${data.gender}\n`,
        `${inlineCode("Birth             :")} ${data.dateOfBirth}\n`,
        `${inlineCode("Death             :")} ${data.dateOfDeath}\n`,
        `${inlineCode("Language          :")} ${data.language}\n`,
        `${inlineCode("Home Town         :")} ${data.homeTown}\n`,
        `${inlineCode("Favourites        :")} ${data.favourites?.toLocaleString("en-US")}\n`,
    ];

    const filtered = filteredDescription(descriptionBuilder.join(""), false);
    const animeList = data.staffData.nodes.filter(
        (media: any) => media.format !== "MANGA" && media.format !== "NOVEL" && media.format !== "ONE_SHOT",
    );

    let animeListString = animeList
        .map((media: any) => {
            const format = media.format === null ? "Unknown" : media.format;
            return `${bold(`[${media.title.romaji}](${media.siteUrl})`)} - (${capitalise(format)})`;
        })
        .join("\n");

    const mangaList = data.staffData.nodes.filter(
        (media: any) => media.format === "MANGA" || media.format === "NOVEL" || media.format === "ONE_SHOT",
    );

    let mangaListString = mangaList
        .map((manga: any) => {
            let format = manga.format === "ONE_SHOT" ? "One Shot" : manga.format;
            format = format === null ? "Unknown" : format;
            return `${bold(`[${manga.title.romaji}](${manga.siteUrl})`)} - (${capitalise(format)})`;
        })
        .join("\n");

    const animeMaxLength = (filtered + animeListString).length;
    const mangaMaxLength = (filtered + mangaListString).length;

    if (animeMaxLength > MAX_DESCRIPTION_LENGTH) {
        const excessLength = animeMaxLength - MAX_DESCRIPTION_LENGTH;
        const itemsToRemove = animeList[0]
            ? Math.ceil(
                  excessLength /
                      ((animeList[0].title.romaji ? animeList[0].title.romaji.length : 0) +
                          animeList[0].siteUrl.length +
                          20),
              )
            : 0;
        animeListString = truncateAnilistIfExceedsDescription(animeListString, itemsToRemove);
    }

    if (mangaMaxLength > MAX_DESCRIPTION_LENGTH) {
        const excessLength = mangaMaxLength - MAX_DESCRIPTION_LENGTH;
        const itemsToRemove = mangaList[0]
            ? Math.ceil(
                  excessLength /
                      ((mangaList[0].title.romaji ? mangaList[0].title.romaji.length : 0) +
                          mangaList[0].siteUrl.length +
                          20),
              )
            : 0;
        mangaListString = truncateAnilistDescription(mangaListString, itemsToRemove);
    }

    return {
        description: filtered,
        animeDescription: `\n${inlineCode("Anime List        :")}\n${animeListString}`,
        mangaDescription: `\n${inlineCode("Manga List        :")}\n${mangaListString}`,
    };
};
