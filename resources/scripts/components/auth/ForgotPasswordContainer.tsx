import tw from 'twin.macro';
import * as React from 'react';
import Reaptcha from 'reaptcha';
import { object, string } from 'yup';
import { Link } from 'react-router-dom';
import useFlash from '@/plugins/useFlash';
import { useStoreState } from 'easy-peasy';
import { httpErrorToHuman } from '@/api/http';
import { Formik, FormikHelpers } from 'formik';
import Field from '@/components/elements/Field';
import { useEffect, useRef, useState } from 'react';
import { Button } from '@/components/elements/button/index';
import LoginFormContainer from '@/components/auth/LoginFormContainer';
import requestPasswordResetEmail from '@/api/auth/requestPasswordResetEmail';

interface Values {
    email: string;
}

export default () => {
    const ref = useRef<Reaptcha>(null);
    const [token, setToken] = useState('');

    const { clearFlashes, addFlash } = useFlash();
    const { enabled: recaptchaEnabled, siteKey } = useStoreState((state) => state.settings.data!.recaptcha);

    useEffect(() => {
        clearFlashes();
    }, []);

    const handleSubmission = ({ email }: Values, { setSubmitting, resetForm }: FormikHelpers<Values>) => {
        clearFlashes();

        // If there is no token in the state yet, request the token and then abort this submit request
        // since it will be re-submitted when the recaptcha data is returned by the component.
        if (recaptchaEnabled && !token) {
            ref.current!.execute().catch((error) => {
                console.error(error);

                setSubmitting(false);
<<<<<<< Updated upstream
                addFlash({ type: 'error', title: 'Erro', message: httpErrorToHuman(error) });
=======
<<<<<<< HEAD
                addFlash({ type: 'danger', title: 'Error', message: httpErrorToHuman(error) });
=======
                addFlash({ type: 'error', title: 'Erro', message: httpErrorToHuman(error) });
>>>>>>> develop
>>>>>>> Stashed changes
            });

            return;
        }

        requestPasswordResetEmail(email, token)
            .then((response) => {
                resetForm();
                addFlash({ type: 'success', title: 'Successo', message: response });
            })
            .catch((error) => {
                console.error(error);
<<<<<<< Updated upstream
                addFlash({ type: 'error', title: 'Erro', message: httpErrorToHuman(error) });
=======
<<<<<<< HEAD
                addFlash({ type: 'danger', title: 'Error', message: httpErrorToHuman(error) });
=======
                addFlash({ type: 'error', title: 'Erro', message: httpErrorToHuman(error) });
>>>>>>> develop
>>>>>>> Stashed changes
            })
            .then(() => {
                setToken('');
                if (ref.current) ref.current.reset();

                setSubmitting(false);
            });
    };

    return (
        <Formik
            onSubmit={handleSubmission}
            initialValues={{ email: '' }}
            validationSchema={object().shape({
                email: string()
                    .email('Deve ser fornecido um endereço de correio E-Mail válido para continuar.')
                    .required('Deve ser fornecido um endereço de E-Mail válido para continuar.'),
            })}
        >
            {({ isSubmitting, setSubmitting, submitForm }) => (
                <LoginFormContainer title={'Pedir redefinição de senha'} css={tw`w-full flex`}>
                    <Field
                        light
                        label={'Email'}
                        description={
                            'Introduza o endereço de e-mail da sua conta para receber instruções sobre como redefinir a sua senha.'
                        }
                        name={'email'}
                        type={'email'}
                    />
                    <div css={tw`mt-6`}>
                        <Button size={Button.Sizes.Large} css={tw`w-full`} type={'submit'} disabled={isSubmitting}>
                            Enviar Email
                        </Button>
                    </div>
                    {recaptchaEnabled && (
                        <Reaptcha
                            ref={ref}
                            size={'invisible'}
                            sitekey={siteKey || '_invalid_key'}
                            onVerify={(response) => {
                                setToken(response);
                                submitForm();
                            }}
                            onExpire={() => {
                                setSubmitting(false);
                                setToken('');
                            }}
                        />
                    )}
                    <div css={tw`mt-6 text-center`}>
                        <Link
                            to={'/auth/login'}
                            css={tw`text-xs text-neutral-500 tracking-wide uppercase no-underline hover:text-neutral-700`}
                        >
                            Voltar ao Login
                        </Link>
                    </div>
                </LoginFormContainer>
            )}
        </Formik>
    );
};
