import http from '@/api/http';

export default async (uuid: string, directory: string, file: string): Promise<void> => {
    await http.post(
        `/api/client/servers/${uuid}/files/decompress`,
        { root: directory, file },
        {
            timeout: 300000,
            timeoutErrorMessage:
                'Parece que este arquivo está demorando muito para ser desarquivado. Depois de concluído, os arquivos desarquivados aparecerão.',
        }
    );
};
