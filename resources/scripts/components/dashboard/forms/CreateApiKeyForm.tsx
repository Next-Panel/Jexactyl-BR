import tw from 'twin.macro';
import { object, string, setLocale } from 'yup';
import { pt } from 'yup-locales';
import React, { useState } from 'react';
import { ApplicationStore } from '@/state';
import styled from 'styled-components/macro';
import { httpErrorToHuman } from '@/api/http';
import { ApiKey } from '@/api/account/getApiKeys';
import createApiKey from '@/api/account/createApiKey';
import { Actions, useStoreActions } from 'easy-peasy';
import { Button } from '@/components/elements/button/index';
import { Field, Form, Formik, FormikHelpers } from 'formik';
import ApiKeyModal from '@/components/dashboard/ApiKeyModal';
import Input, { Textarea } from '@/components/elements/Input';
import SpinnerOverlay from '@/components/elements/SpinnerOverlay';
import FormikFieldWrapper from '@/components/elements/FormikFieldWrapper';

interface Values {
    description: string;
    allowedIps: string;
}

const CustomTextarea = styled(Textarea)`
    ${tw`h-32`}
`;

export default ({ onKeyCreated }: { onKeyCreated: (key: ApiKey) => void }) => {
    const [apiKey, setApiKey] = useState('');
    const { addError, clearFlashes } = useStoreActions((actions: Actions<ApplicationStore>) => actions.flashes);
    setLocale(pt);

    const submit = (values: Values, { setSubmitting, resetForm }: FormikHelpers<Values>) => {
        clearFlashes('account');
        createApiKey(values.description, values.allowedIps)
            .then(({ secretToken, ...key }) => {
                resetForm();
                setSubmitting(false);
                setApiKey(`${key.identifier}${secretToken}`);
                onKeyCreated(key);
            })
            .catch((error) => {
                console.error(error);

                addError({ key: 'account', message: httpErrorToHuman(error) });
                setSubmitting(false);
            });
    };

    return (
        <>
            <ApiKeyModal visible={apiKey.length > 0} onModalDismissed={() => setApiKey('')} apiKey={apiKey} />
            <Formik
                onSubmit={submit}
                initialValues={{ description: '', allowedIps: '' }}
                validationSchema={object().shape({
                    allowedIps: string(),
                    description: string().required().min(4),
                })}
            >
                {({ isSubmitting }) => (
                    <Form>
                        <SpinnerOverlay visible={isSubmitting} />
                        <FormikFieldWrapper
                            label={'Descrição'}
                            name={'description'}
                            description={'descrição desta chave API.'}
                            css={tw`mb-6`}
                        >
                            <Field name={'description'} as={Input} />
                        </FormikFieldWrapper>
                        <FormikFieldWrapper
                            label={'IPs permitidos'}
                            name={'allowedIps'}
                            description={
                                'Deixe em branco para permitir que qualquer endereço IP possa utilizar esta chave API, caso contrário, forneça cada endereço IP numa nova linha.'
                            }
                        >
                            <Field name={'allowedIps'} as={CustomTextarea} />
                        </FormikFieldWrapper>
                        <div css={tw`flex justify-end mt-6`}>
                            <Button>Criar</Button>
                        </div>
                    </Form>
                )}
            </Formik>
        </>
    );
};
