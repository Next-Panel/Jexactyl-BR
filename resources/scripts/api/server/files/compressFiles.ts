import http from '@/api/http';
import { rawDataToFileObject } from '@/api/transformers';
import { FileObject } from '@/api/server/files/loadDirectory';

export default async (uuid: string, directory: string, files: string[]): Promise<FileObject> => {
    const { data } = await http.post(
        `/api/client/servers/${uuid}/files/compress`,
        { root: directory, files },
        {
            timeout: 60000,
            timeoutErrorMessage:
                'Parece que este arquivo está demorando muito para ser gerado. Ele aparecerá de uma vez concluído.',
        }
    );

    return rawDataToFileObject(data);
};
