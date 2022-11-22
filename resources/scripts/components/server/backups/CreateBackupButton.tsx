import tw from 'twin.macro';
import useFlash from '@/plugins/useFlash';
import Can from '@/components/elements/Can';
import { boolean, object, string } from 'yup';
import { ServerContext } from '@/state/server';
import Field from '@/components/elements/Field';
import React, { useEffect, useState } from 'react';
import { Textarea } from '@/components/elements/Input';
import getServerBackups from '@/api/swr/getServerBackups';
import { Button } from '@/components/elements/button/index';
import FormikSwitch from '@/components/elements/FormikSwitch';
import FlashMessageRender from '@/components/FlashMessageRender';
import Modal, { RequiredModalProps } from '@/components/elements/Modal';
import createServerBackup from '@/api/server/backups/createServerBackup';
import FormikFieldWrapper from '@/components/elements/FormikFieldWrapper';
import { Field as FormikField, Form, Formik, FormikHelpers, useFormikContext } from 'formik';

interface Values {
    name: string;
    ignored: string;
    isLocked: boolean;
}

const ModalContent = ({ ...props }: RequiredModalProps) => {
    const { isSubmitting } = useFormikContext<Values>();

    return (
        <Modal {...props} showSpinnerOverlay={isSubmitting}>
            <Form>
                <FlashMessageRender byKey={'backups:create'} css={tw`mb-4`} />
                <h2 css={tw`text-2xl mb-6`}>Crie backup do servidor</h2>
                <Field
                    name={'name'}
                    label={'Nome de backup'}
                    description={'Se fornecido, o nome que deve ser usado para fazer referência a este backup.'}
                />
                <div css={tw`mt-6`}>
                    <FormikFieldWrapper
                        name={'ignored'}
                        label={'Arquivos e diretórios ignorados'}
                        description={`
                            Digite os arquivos ou pastas para ignorar enquanto gera esse backup.Deixar em branco para usar
                            O conteúdo do arquivo .pteroignore na raiz do diretório do servidor, se estiver presente.
                            A correspondência curinga de arquivos e pastas é suportada, além de negar uma regra por
                            Prefixando o caminho com um ponto de exclamação.
                        `}
                    >
                        <FormikField as={Textarea} name={'ignored'} rows={6} />
                    </FormikFieldWrapper>
                </div>
                <Can action={'backup.delete'}>
                    <div css={tw`mt-6 bg-neutral-700 border border-neutral-800 shadow-inner p-4 rounded`}>
                        <FormikSwitch
                            name={'isLocked'}
                            label={'Trancado'}
                            description={'Impede que esse backup seja excluído até ser desbloqueado explicitamente.'}
                        />
                    </div>
                </Can>
                <div css={tw`flex justify-end mt-6`}>
                    <Button type={'submit'} disabled={isSubmitting}>
                        Iniciar backup
                    </Button>
                </div>
            </Form>
        </Modal>
    );
};

export default () => {
    const uuid = ServerContext.useStoreState((state) => state.server.data!.uuid);
    const { clearFlashes, clearAndAddHttpError } = useFlash();
    const [visible, setVisible] = useState(false);
    const { mutate } = getServerBackups();

    useEffect(() => {
        clearFlashes('backups:create');
    }, [visible]);

    const submit = (values: Values, { setSubmitting }: FormikHelpers<Values>) => {
        clearFlashes('backups:create');
        createServerBackup(uuid, values)
            .then((backup) => {
                mutate(
                    (data) => ({ ...data, items: data.items.concat(backup), backupCount: data.backupCount + 1 }),
                    false
                );
                setVisible(false);
            })
            .catch((error) => {
                clearAndAddHttpError({ key: 'backups:create', error });
                setSubmitting(false);
            });
    };

    return (
        <>
            {visible && (
                <Formik
                    onSubmit={submit}
                    initialValues={{ name: '', ignored: '', isLocked: false }}
                    validationSchema={object().shape({
                        name: string().max(191),
                        ignored: string(),
                        isLocked: boolean(),
                    })}
                >
                    <ModalContent appear visible={visible} onDismissed={() => setVisible(false)} />
                </Formik>
            )}
            <Button css={tw`w-full sm:w-auto`} onClick={() => setVisible(true)}>
                Criar backup
            </Button>
        </>
    );
};
